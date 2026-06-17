use std::{
    cell::{Cell, RefCell},
    mem::ManuallyDrop,
    rc::{Rc, Weak},
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicPtr, Ordering},
    },
};

use anyhow::Context;
use windows::{
    UI::Composition::{CompositionBackfaceVisibility, Compositor, ContainerVisual, Desktop::DesktopWindowTarget, SpriteVisual},
    Win32::{
        Foundation::{COLORREF, ERROR_SUCCESS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, RECT, SetLastError, WIN32_ERROR, WPARAM},
        Graphics::{
            Dwm::{
                DWM_SYSTEMBACKDROP_TYPE, DWMWA_CAPTION_COLOR, DWMWA_COLOR_NONE, DWMWA_EXTENDED_FRAME_BOUNDS, DWMWA_SYSTEMBACKDROP_TYPE,
                DWMWA_USE_IMMERSIVE_DARK_MODE, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute, DwmSetWindowAttribute,
            },
            Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow, RDW_INVALIDATE, RDW_NOERASE, RDW_NOFRAME, RedrawWindow},
        },
        System::{
            Threading::{AttachThreadInput, GetCurrentThreadId},
            WinRT::Composition::ICompositorDesktopInterop,
        },
        UI::{
            Controls::MARGINS,
            HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
            Input::KeyboardAndMouse::SetActiveWindow,
            WindowsAndMessaging::{
                BringWindowToTop, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateIconFromResourceEx, CreateWindowExW, DefWindowProcW,
                DestroyWindow, GWL_STYLE, GetClientRect, GetForegroundWindow, GetPropW, GetWindowLongPtrW, GetWindowThreadProcessId, HMENU,
                ICON_BIG, ICON_SMALL, IsHungAppWindow, IsIconic, IsZoomed, LR_DEFAULTCOLOR, PostMessageW, RegisterClassExW, RemovePropW,
                SC_MAXIMIZE, SC_MINIMIZE, SC_RESTORE, SM_CXICON, SM_CXPADDEDBORDER, SM_CXSMICON, SM_CYICON, SM_CYSIZEFRAME, SM_CYSMICON,
                SW_SHOW, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SendMessageW,
                SetCursor, SetForegroundWindow, SetPropW, SetWindowLongPtrW, SetWindowPos, SetWindowTextW, ShowWindow, TPM_RETURNCMD,
                TPM_RIGHTBUTTON, TrackPopupMenu, USER_DEFAULT_SCREEN_DPI, WM_CLOSE, WM_NCCREATE, WM_NCDESTROY, WM_NULL, WM_SETICON,
                WM_SYSCOMMAND, WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_OVERLAPPEDWINDOW, WS_THICKFRAME,
            },
        },
    },
};
use windows_core::{HSTRING, Interface, PCWSTR, Result as WinResult, w};

use super::{
    appearance::{self, Appearance},
    cursor::{Cursor, CursorIcon},
    event_loop::EventLoop,
    geometry::{LogicalPoint, LogicalRect, LogicalSize, PhysicalPoint},
    pointer::{PointerButton, PointerClickCounter},
    screen::{self, ScreenInfo},
    strings::copy_from_utf8_string,
    system_menu::{seed_system_menu, sync_system_menu_state},
    utils,
    window_api::{WindowParams, WindowStyle, WindowTitleBarKind},
};

#[derive(Clone, Copy)]
enum StyleFlag {
    Resizable,
    Minimizable,
    Maximizable,
}

impl StyleFlag {
    const fn bit(self) -> u32 {
        match self {
            Self::Resizable => WS_THICKFRAME.0,
            Self::Minimizable => WS_MINIMIZEBOX.0,
            Self::Maximizable => WS_MAXIMIZEBOX.0,
        }
    }
}

/// cbindgen:ignore
const WINDOW_PTR_PROP_NAME: PCWSTR = w!("KDT_WINDOW_PTR");
/// cbindgen:ignore
const WNDCLASS_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32Window");

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WindowId(pub isize);

/// Per-DPI metrics for chrome and hit-test code. Seeded in `initialize_window`,
/// refreshed on `WM_DPICHANGED`. Fields not cached here (e.g. `SM_CYSIZE`)
/// are queried via `GetSystemMetricsForDpi` at point of use.
#[derive(Clone, Copy)]
pub(crate) struct DpiMetrics {
    #[allow(unused)]
    pub dpi: u32,
    pub scale: f32,
    pub padded_border: i32,
    pub size_frame: i32,
}

impl DpiMetrics {
    #[allow(clippy::cast_precision_loss)]
    fn for_dpi(dpi: u32) -> Self {
        let dpi_u16 = u16::try_from(dpi).unwrap_or(u16::MAX); // OS DPI range fits u16
        Self {
            dpi,
            scale: f32::from(dpi_u16) / (USER_DEFAULT_SCREEN_DPI as f32),
            padded_border: unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) },
            size_frame: unsafe { GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) },
        }
    }
}

#[allow(clippy::struct_field_names)]
pub struct Window {
    id: WindowId,
    hwnd: AtomicPtr<core::ffi::c_void>,
    compositor: Compositor,
    composition_target: RefCell<Option<DesktopWindowTarget>>,
    composition_root: RefCell<Option<ContainerVisual>>,
    backdrop_layer: RefCell<Option<ContainerVisual>>,
    content_layer: RefCell<Option<ContainerVisual>>,
    min_size: Cell<Option<LogicalSize>>,
    origin: Cell<LogicalPoint>,
    size: Cell<LogicalSize>,
    style: RefCell<WindowStyle>,
    pointer_in_window: AtomicBool,
    /// True while we hold an explicit `SetCapture` taken for a consumed non-client press
    /// (see `on_pointerdown`). Lets `on_pointerup` release only *our* capture and never the
    /// one `DefWindowProc`'s move/resize modal loop sets for itself (releasing that would
    /// cancel the drag and snap the window back).
    self_captured_pointer: AtomicBool,
    /// Bitmask (bits match `PointerButton as u32`) of buttons we've delivered a `PointerDown`
    /// for but not yet a `PointerUp`. Windows occasionally drops a `WM_POINTERUP` (e.g. a
    /// release over a caption button), leaving the button stuck "pressed". On the next pointer
    /// update we reconcile this against the OS-reported pressed set and synthesise the missing
    /// `PointerUp` for any button that's no longer down.
    pressed_pointer_buttons: Cell<u32>,
    cached_dpi_metrics: Cell<DpiMetrics>,
    system_menu: Cell<HMENU>,
    immersive_dark: Cell<bool>,
    pointer_click_counter: RefCell<PointerClickCounter>,
    cursor: RefCell<Option<Cursor>>,
    backdrop_tint: RefCell<Option<SpriteVisual>>,
    event_loop: Weak<EventLoop>,
}

impl Window {
    pub fn new(window_id: WindowId, event_loop: Weak<EventLoop>, compositor: Compositor) -> anyhow::Result<Self> {
        static WNDCLASS_INIT: OnceLock<u16> = OnceLock::new();
        let wndclass_size = size_of::<WNDCLASSEXW>().try_into()?;
        let _ = WNDCLASS_INIT.get_or_init(|| {
            let wndclass = WNDCLASSEXW {
                cbSize: wndclass_size,
                hInstance: crate::get_dll_instance(),
                lpszClassName: WNDCLASS_NAME,
                lpfnWndProc: Some(wndproc),
                style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            };
            unsafe { RegisterClassExW(&raw const wndclass) }
        });
        let window = Self {
            id: window_id,
            hwnd: AtomicPtr::default(),
            compositor,
            composition_target: RefCell::new(None),
            composition_root: RefCell::new(None),
            backdrop_layer: RefCell::new(None),
            content_layer: RefCell::new(None),
            min_size: Cell::new(None),
            origin: Cell::default(),
            size: Cell::new(LogicalSize::new(0.0, 0.0)),
            style: RefCell::default(),
            pointer_in_window: AtomicBool::new(false),
            self_captured_pointer: AtomicBool::new(false),
            pressed_pointer_buttons: Cell::new(0),
            cached_dpi_metrics: Cell::new(DpiMetrics::for_dpi(USER_DEFAULT_SCREEN_DPI)),
            system_menu: Cell::new(HMENU::default()),
            immersive_dark: Cell::new(false),
            pointer_click_counter: RefCell::new(PointerClickCounter::new()),
            cursor: RefCell::new(None),
            backdrop_tint: RefCell::new(None),
            event_loop,
        };
        Ok(window)
    }

    pub fn create(window: &Rc<Self>, creation_params: &WindowParams) -> anyhow::Result<()> {
        let instance = crate::get_dll_instance();
        window.origin.set(creation_params.origin);
        window.size.set(creation_params.size);
        window.style.replace(creation_params.style);
        let title = copy_from_utf8_string(&creation_params.title)?;
        unsafe {
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                WNDCLASS_NAME,
                &title,
                WS_OVERLAPPEDWINDOW,
                0, // CW_USEDEFAULT: i32 = -2147483648i32
                0, // CW_USEDEFAULT: i32 = -2147483648i32
                1,
                1,
                None,
                None,
                Some(instance),
                Some(Rc::downgrade(window).into_raw().cast()),
            )?;
        }
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> WindowId {
        self.id
    }

    #[inline]
    pub(crate) fn hwnd(&self) -> HWND {
        HWND(self.hwnd.load(Ordering::Acquire))
    }

    #[inline]
    pub(crate) fn add_visual(&self) -> anyhow::Result<SpriteVisual> {
        let sprite_visual = self.compositor.CreateSpriteVisual()?;
        self.content_layer
            .borrow()
            .as_ref()
            .context("Window has not been created yet")?
            .Children()?
            .InsertAtTop(&sprite_visual)?;
        Ok(sprite_visual)
    }

    pub fn get_client_size(&self) -> anyhow::Result<LogicalSize> {
        let mut rect = RECT::default();
        unsafe { GetClientRect(self.hwnd(), &raw mut rect)? };
        // According to the documentation (https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclientrect):
        //   "The left and top members are zero. The right and bottom members contain the width and height of the window."
        Ok(LogicalSize::from_physical(rect.right, rect.bottom, self.get_scale()))
    }

    pub(crate) fn get_physical_rect(&self) -> anyhow::Result<RECT> {
        let mut rect = RECT::default();
        unsafe {
            DwmGetWindowAttribute(
                self.hwnd(),
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&raw mut rect).cast(),
                size_of::<RECT>().try_into()?,
            )?;
        };
        Ok(rect)
    }

    pub fn get_rect(&self) -> anyhow::Result<LogicalRect> {
        let rect = self.get_physical_rect()?;
        let scale = self.get_scale();
        let origin = LogicalPoint::from_physical(rect.left, rect.top, scale);
        let size = LogicalSize::from_physical(rect.right - rect.left, rect.bottom - rect.top, scale);
        Ok(LogicalRect { origin, size })
    }

    #[must_use]
    pub const fn get_scale(&self) -> f32 {
        self.dpi_metrics().scale
    }

    pub(crate) const fn dpi_metrics(&self) -> DpiMetrics {
        self.cached_dpi_metrics.get()
    }

    pub(crate) fn set_dpi_metrics(&self, dpi: u32) {
        self.cached_dpi_metrics.set(DpiMetrics::for_dpi(dpi));
    }

    pub fn get_screen_info(&self) -> anyhow::Result<ScreenInfo> {
        let hmonitor = unsafe { MonitorFromWindow(self.hwnd(), MONITOR_DEFAULTTONEAREST) };
        screen::get_screen_info(hmonitor)
    }

    #[must_use]
    pub fn has_custom_title_bar(&self) -> bool {
        matches!(self.style.borrow().title_bar_kind, WindowTitleBarKind::Custom)
    }

    #[must_use]
    pub fn is_maximized(&self) -> bool {
        unsafe { IsZoomed(self.hwnd()) }.as_bool()
    }

    #[must_use]
    pub fn is_minimized(&self) -> bool {
        unsafe { IsIconic(self.hwnd()) }.as_bool()
    }

    #[must_use]
    pub fn is_resizable(&self) -> bool {
        self.style.borrow().is_resizable
    }

    #[must_use]
    pub fn is_minimizable(&self) -> bool {
        self.style.borrow().is_minimizable
    }

    #[must_use]
    pub fn is_maximizable(&self) -> bool {
        self.style.borrow().is_maximizable
    }

    pub(crate) fn extend_content_into_titlebar(&self) -> WinResult<()> {
        if utils::is_windows_11_build_22000_or_higher() {
            let colorref = COLORREF(DWMWA_COLOR_NONE);
            unsafe {
                // if we want to extend content into the titlebar area, it makes sense to remove any color from it
                DwmSetWindowAttribute(
                    self.hwnd(),
                    DWMWA_CAPTION_COLOR,
                    (&raw const colorref).cast(),
                    size_of::<COLORREF>().try_into()?,
                )?;
            }
        }
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        unsafe { DwmExtendFrameIntoClientArea(self.hwnd(), &raw const margins) }
    }

    pub(crate) fn apply_system_backdrop(&self) -> WinResult<()> {
        if utils::is_windows_11_build_22621_or_higher() {
            let backdrop: DWM_SYSTEMBACKDROP_TYPE = self.style.borrow().system_backdrop_type.to_system();
            unsafe {
                DwmSetWindowAttribute(
                    self.hwnd(),
                    DWMWA_SYSTEMBACKDROP_TYPE,
                    (&raw const backdrop).cast(),
                    size_of::<DWM_SYSTEMBACKDROP_TYPE>().try_into()?,
                )?;
            }
        }
        Ok(())
    }

    pub fn maximize(&self) {
        if !self.style.borrow().is_maximizable {
            return;
        }
        self.send_system_command(SC_MAXIMIZE);
    }

    pub fn minimize(&self) {
        if !self.style.borrow().is_minimizable {
            return;
        }
        self.send_system_command(SC_MINIMIZE);
    }

    pub fn restore(&self) {
        self.send_system_command(SC_RESTORE);
    }

    #[inline]
    pub(crate) const fn system_menu(&self) -> HMENU {
        self.system_menu.get()
    }

    /// Apply enable state to the cached `HMENU`. Called from the
    /// `WM_INITMENUPOPUP` arm so we run after `DefWindowProc`'s default
    /// auto-gray.
    pub(crate) fn sync_system_menu(&self) {
        sync_system_menu_state(self.system_menu(), &self.style.borrow(), self.is_maximized(), self.is_minimized());
    }

    /// Show the system menu at `screen_pt` and dispatch the user's choice via
    /// `WM_SYSCOMMAND`. Enable state is applied by the `WM_INITMENUPOPUP` arm
    /// during `TrackPopupMenu`'s popup-init phase.
    pub(crate) fn show_system_menu(&self, screen_pt: PhysicalPoint) -> anyhow::Result<()> {
        let hwnd = self.hwnd();
        let h_menu = self.system_menu();
        if h_menu.is_invalid() {
            anyhow::bail!("system menu not initialized for this window");
        }

        let _ = unsafe { SetForegroundWindow(hwnd) };
        // Theme the popup from this window's own state; the mode is process-global,
        // so force it only around TrackPopupMenu and restore it afterward.
        let appearance = if self.immersive_dark.get() {
            Appearance::Dark
        } else {
            Appearance::Light
        };
        // TrackPopupMenu with TPM_RETURNCMD returns 0 for both cancel and
        // failure; distinguish via GetLastError (read inside the closure, before restore).
        let (cmd, last_error) = appearance::with_preferred_app_mode(appearance, || {
            unsafe { SetLastError(WIN32_ERROR(0)) };
            let cmd = unsafe {
                TrackPopupMenu(
                    h_menu,
                    TPM_RIGHTBUTTON | TPM_RETURNCMD,
                    screen_pt.x.0,
                    screen_pt.y.0,
                    None,
                    hwnd,
                    None,
                )
            };
            (cmd, unsafe { GetLastError() })
        });
        // Docs-recommended post-show null-message flush.
        let _ = unsafe { PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) };

        // `TPM_RETURNCMD` makes `TrackPopupMenu` return the selected menu-item ID
        // or 0 for cancel-OR-failure. Distinguish via GetLastError.
        if cmd.0 == 0 && last_error != ERROR_SUCCESS {
            anyhow::bail!("TrackPopupMenu failed: {last_error:?}");
        }
        // Mask to the standard SC_* range before forwarding.
        if let Ok(cmd_id) = u32::try_from(cmd.0)
            && cmd_id != 0
        {
            self.send_system_command(cmd_id & 0xFFF0);
        }
        Ok(())
    }

    #[inline]
    fn send_system_command(&self, command: u32) {
        // Route through the system-command path to preserve standard shell
        // transitions (including minimize/maximize/restore animations).
        let _ = unsafe { SendMessageW(self.hwnd(), WM_SYSCOMMAND, Some(WPARAM(command as usize)), Some(LPARAM(0))) };
    }

    pub fn show(&self) {
        let _ = unsafe { ShowWindow(self.hwnd(), SW_SHOW) };
    }

    /// Forcibly brings the window to the foreground and gives it keyboard focus.
    ///
    /// Windows refuses `SetForegroundWindow` for a process that does not own the
    /// current foreground window (e.g. when the app is launched from a terminal or
    /// IDE rather than the shell), and merely flashes the taskbar button instead.
    /// Temporarily attaching our input queue to the foreground window's thread lifts
    /// that restriction for the duration of the call. This deliberately steals focus
    /// from whatever the user is doing, so it should be used sparingly.
    pub fn force_focus(&self) {
        let hwnd = self.hwnd();

        // Restore first if minimized, otherwise the window would be activated but stay
        // iconic. Route through WM_SYSCOMMAND (SC_RESTORE) like `restore()` rather than
        // `ShowWindow(SW_RESTORE)`: it is synchronous on the window's UI thread (ShowWindow
        // is async when issued cross-thread) and preserves the standard restore animation.
        if self.is_minimized() {
            self.send_system_command(SC_RESTORE);
        }

        unsafe {
            let foreground = GetForegroundWindow();
            let foreground_thread = GetWindowThreadProcessId(foreground, None);
            let current_thread = GetCurrentThreadId();
            // Attaching our input queue to the foreground thread is what lets
            // `SetForegroundWindow` bypass the foreground lock. After the attach, though,
            // `SetForegroundWindow`/`BringWindowToTop` deliver synchronously to the attached
            // thread; if its process is hung, that would block our UI thread indefinitely. Skip
            // the attach when the foreground window is unresponsive and fall back to the taskbar
            // flash. A small TOCTOU gap remains (the target could hang after this check); it is
            // inherent to this approach.
            let attached = foreground_thread != 0
                && foreground_thread != current_thread
                && !IsHungAppWindow(foreground).as_bool()
                && AttachThreadInput(current_thread, foreground_thread, true).as_bool();

            let _ = SetForegroundWindow(hwnd);
            let _ = BringWindowToTop(hwnd);
            let _ = SetActiveWindow(hwnd);

            if attached {
                let _ = AttachThreadInput(current_thread, foreground_thread, false);
            }
        }
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        unsafe { SetCursor(Some(cursor.as_native())) };
        self.cursor.replace(Some(cursor));
    }

    pub fn set_backdrop_tint(&self, color: u32, opacity: f32) -> anyhow::Result<()> {
        let backdrop_tint = self.backdrop_tint.borrow();
        let backdrop_visual = backdrop_tint.as_ref().context("Window has not been created yet")?;
        let [a, r, g, b] = color.to_be_bytes();
        let backdrop_color = windows::UI::Color { A: a, R: r, G: g, B: b };
        let backdrop_brush = self.compositor.CreateColorBrushWithColor(backdrop_color)?;
        backdrop_visual.SetBrush(&backdrop_brush)?;
        backdrop_visual.SetOpacity(opacity)?;
        backdrop_visual.SetIsVisible(true)?;
        Ok(())
    }

    pub fn remove_backdrop_tint(&self) -> anyhow::Result<()> {
        if let Some(backdrop_visual) = self.backdrop_tint.borrow().as_ref() {
            backdrop_visual.SetIsVisible(false)?;
        }
        Ok(())
    }

    /// Win11 maximize-overhang inset in physical pixels: `SM_CYSIZEFRAME`
    /// (DPI-aware) when resizable, maximized, and a custom title bar;
    /// `0` otherwise.
    pub(crate) fn max_chrome_y(&self) -> i32 {
        if !self.is_resizable() || !self.has_custom_title_bar() || !self.is_maximized() {
            return 0;
        }
        self.dpi_metrics().size_frame
    }

    /// Offsets `content_layer` by `top_offset_px` so content aligns with the
    /// visible monitor edge when maximized (composition origin is at the
    /// off-monitor window-rect top-left). Does not commit; fires `CommitNeeded`.
    pub(crate) fn set_content_top_offset(&self, top_offset_px: i32) -> anyhow::Result<()> {
        if let Some(layer) = self.content_layer.borrow().as_ref() {
            #[allow(clippy::cast_precision_loss)]
            layer.SetOffset(windows_numerics::Vector3 {
                X: 0.0,
                Y: top_offset_px as f32,
                Z: 0.0,
            })?;
        }
        Ok(())
    }

    pub(crate) fn refresh_cursor(&self) -> WinResult<()> {
        let current_cursor = self.cursor.borrow();
        if let Some(cursor) = current_cursor.as_ref() {
            unsafe { SetCursor(Some(cursor.as_native())) };
        } else {
            let arrow_cursor = Cursor::load_from_system(CursorIcon::Arrow)?;
            unsafe { SetCursor(Some(arrow_cursor.as_native())) };
        }
        Ok(())
    }

    pub fn set_immersive_dark_mode(&self, enabled: bool) -> WinResult<()> {
        if !utils::is_windows_11_build_22000_or_higher() {
            return Ok(());
        }
        if self.immersive_dark.get() == enabled {
            return Ok(());
        }
        let enablement = windows_core::BOOL::from(enabled);
        unsafe {
            DwmSetWindowAttribute(
                self.hwnd(),
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&raw const enablement).cast(),
                size_of::<windows_core::BOOL>().try_into()?,
            )?;
        }
        self.immersive_dark.set(enabled);
        Ok(())
    }

    pub fn set_position(&self, origin: LogicalPoint, size: LogicalSize) -> WinResult<()> {
        let scale = self.get_scale();
        let physical_origin = origin.to_physical(scale);
        let physical_size = size.to_physical(scale);
        unsafe {
            SetWindowPos(
                self.hwnd(),
                None,
                physical_origin.x.0,
                physical_origin.y.0,
                physical_size.width.0,
                physical_size.height.0,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOZORDER,
            )
        }
    }

    pub fn set_title(&self, title: &HSTRING) -> WinResult<()> {
        unsafe { SetWindowTextW(self.hwnd(), title) }
    }

    pub fn set_is_resizable(&self, value: bool) -> anyhow::Result<()> {
        // Update cache before FRAMECHANGED so the synchronous NCCALCSIZE reads fresh state.
        self.style.borrow_mut().is_resizable = value;
        self.update_style_flag(StyleFlag::Resizable, value)?;
        Ok(())
    }

    pub fn set_is_minimizable(&self, value: bool) -> anyhow::Result<()> {
        self.style.borrow_mut().is_minimizable = value;
        self.update_style_flag(StyleFlag::Minimizable, value)?;
        Ok(())
    }

    pub fn set_is_maximizable(&self, value: bool) -> anyhow::Result<()> {
        self.style.borrow_mut().is_maximizable = value;
        self.update_style_flag(StyleFlag::Maximizable, value)?;
        Ok(())
    }

    fn update_style_flag(&self, flag: StyleFlag, value: bool) -> WinResult<()> {
        let hwnd = self.hwnd();
        let bit = flag.bit();
        let current: u32 = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) }.try_into().unwrap();
        let new_style = if value { current | bit } else { current & !bit };
        if new_style == current {
            return Ok(());
        }
        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, new_style.try_into().unwrap()) };
        // SWP_FRAMECHANGED forces the cached non-client area to be recomputed so the
        // new style takes effect; size/position/z-order are intentionally preserved.
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE,
            )?;
        }
        Ok(())
    }

    #[must_use]
    pub const fn get_min_size(&self) -> Option<LogicalSize> {
        self.min_size.get()
    }

    pub fn set_min_size(&self, size: LogicalSize) {
        self.min_size.set(Some(size));
    }

    #[inline]
    pub(crate) fn is_pointer_in_window(&self) -> bool {
        self.pointer_in_window.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn set_is_pointer_in_window(&self, value: bool) {
        self.pointer_in_window.store(value, Ordering::Relaxed);
    }

    #[inline]
    pub(crate) fn set_self_captured_pointer(&self, value: bool) {
        self.self_captured_pointer.store(value, Ordering::Relaxed);
    }

    /// Atomically clears the self-capture flag, returning its previous value. True means we
    /// took the capture ourselves and are responsible for releasing it.
    #[inline]
    pub(crate) fn take_self_captured_pointer(&self) -> bool {
        self.self_captured_pointer.swap(false, Ordering::Relaxed)
    }

    /// Bitmask of buttons we've delivered a `PointerDown` for without a matching `PointerUp`.
    #[inline]
    pub(crate) const fn pressed_pointer_buttons(&self) -> u32 {
        self.pressed_pointer_buttons.get()
    }

    #[inline]
    pub(crate) fn mark_pointer_button_pressed(&self, button: PointerButton) {
        self.pressed_pointer_buttons.set(self.pressed_pointer_buttons.get() | button as u32);
    }

    #[inline]
    pub(crate) fn mark_pointer_button_released(&self, button: PointerButton) {
        self.pressed_pointer_buttons
            .set(self.pressed_pointer_buttons.get() & !(button as u32));
    }

    #[inline]
    pub(crate) fn with_mut_pointer_click_counter<R>(&self, f: impl FnOnce(&mut PointerClickCounter) -> R) -> R {
        let mut pointer_click_counter = self.pointer_click_counter.borrow_mut();
        f(&mut pointer_click_counter)
    }

    pub fn request_redraw(&self) -> WinResult<()> {
        unsafe { RedrawWindow(Some(self.hwnd()), None, None, RDW_INVALIDATE | RDW_NOFRAME | RDW_NOERASE) }.ok()
    }

    pub fn request_close(&self) -> WinResult<()> {
        unsafe { PostMessageW(Some(self.hwnd()), WM_CLOSE, WPARAM::default(), LPARAM::default()) }
    }

    pub fn destroy(&self) -> WinResult<()> {
        unsafe { DestroyWindow(self.hwnd()) }
    }

    pub fn set_icon(&self, bytes: &[u8]) -> anyhow::Result<()> {
        const ICON_VER: u32 = 0x0003_0000;
        let hwnd = self.hwnd();
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let set_icon_worker = |cx, cy, kind| -> WinResult<()> {
            unsafe {
                let cx = GetSystemMetricsForDpi(cx, dpi);
                let cy = GetSystemMetricsForDpi(cy, dpi);
                let icon = CreateIconFromResourceEx(bytes, true, ICON_VER, cx, cy, LR_DEFAULTCOLOR)?;
                SendMessageW(hwnd, WM_SETICON, Some(WPARAM(kind as _)), Some(LPARAM(icon.0 as _)));
            }
            Ok(())
        };
        set_icon_worker(SM_CXSMICON, SM_CYSMICON, ICON_SMALL)?;
        set_icon_worker(SM_CXICON, SM_CYICON, ICON_BIG)?;
        Ok(())
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        log::trace!("Window has been dropped");
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if hwnd.0.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    // WM_NCCREATE is sent before CreateWindowEx returns and is used to setup the new window
    if msg == WM_NCCREATE {
        return match on_nccreate(hwnd, lparam) {
            Ok(()) => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            Err(err) => {
                log::error!("WM_NCCREATE failed: {err}");
                LRESULT(windows::Win32::Foundation::FALSE.0 as _)
            }
        };
    }

    // WM_NCDESTROY is a special case: this is when we must clean up the extra resources used by the window
    if msg == WM_NCDESTROY {
        if let Ok(raw) = unsafe { RemovePropW(hwnd, WINDOW_PTR_PROP_NAME) } {
            // Reclaim and drop the `Weak` leaked via `into_raw` in `on_nccreate`,
            // before the HWND is recycled.
            drop(unsafe { Weak::from_raw(raw.0.cast::<Window>()) });
        }
        return LRESULT(0);
    }

    let raw = unsafe { GetPropW(hwnd, WINDOW_PTR_PROP_NAME).0.cast::<Window>() };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    // we reuse the weak reference on every iteration of the event loop, so we don't drop it here (see above)
    let this = ManuallyDrop::new(unsafe { Weak::from_raw(raw) });
    match this.upgrade() {
        Some(window) if hwnd == window.hwnd() => {
            let event_loop = window.event_loop.upgrade().expect("event loop has been dropped");
            event_loop.window_proc(&window, msg, wparam, lparam)
        }
        _ => {
            log::error!("the window pointer was incorrect");
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }
}

fn on_nccreate(hwnd: HWND, lparam: LPARAM) -> anyhow::Result<()> {
    let create_struct = unsafe { (lparam.0 as *mut CREATESTRUCTW).as_mut() }.context("CREATESTRUCTW is null")?;

    let window_ptr = create_struct.lpCreateParams.cast_const().cast::<Window>();
    let john_weak = unsafe { Weak::from_raw(window_ptr) };

    let window = john_weak.upgrade().context("failed to upgrade the window weak reference")?;
    initialize_window(&window, hwnd).context("failed to initialize the window")?;

    unsafe {
        SetPropW(
            hwnd,
            WINDOW_PTR_PROP_NAME,
            Some(HANDLE(Weak::into_raw(john_weak).cast_mut().cast())),
        )
    }
    .context("failed to set the window property")
}

fn initialize_window(window: &Window, hwnd: HWND) -> anyhow::Result<()> {
    window.hwnd.store(hwnd.0, Ordering::Release);
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    window.set_dpi_metrics(dpi);
    // Materialise the system-menu copy while WS_SYSMENU is still set, before
    // the style narrow below clears it. Cache the HMENU for reuse on every
    // show — Win32 owns the lifetime (destroyed with the window).
    if window.has_custom_title_bar() {
        let h_menu = seed_system_menu(hwnd).context("failed to seed system menu")?;
        window.system_menu.set(h_menu);
    }
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, window.style.borrow().to_system().0 as _) };
    window.set_position(window.origin.get(), window.size.get())?;
    initialize_content(window, hwnd).context("failed to initialize the content")?;
    window.set_cursor(Cursor::load_from_system(CursorIcon::Arrow)?);
    Ok(())
}

fn initialize_content(window: &Window, hwnd: HWND) -> anyhow::Result<()> {
    let compositor_interop: ICompositorDesktopInterop = window.compositor.cast()?;
    let desktop_window_target = unsafe { compositor_interop.CreateDesktopWindowTarget(hwnd, false) }?;

    let root_visual = window.compositor.CreateContainerVisual()?;
    root_visual.SetBackfaceVisibility(CompositionBackfaceVisibility::Hidden)?;

    let backdrop_layer = window.compositor.CreateContainerVisual()?;
    let content_layer = window.compositor.CreateContainerVisual()?;

    // Track HWND client size on root and backdrop layers.
    // `content_layer` is excluded: the ANGLE visual sets its own absolute
    // size in `resize_surface` and no other child reads the parent's size.
    root_visual.SetRelativeSizeAdjustment(windows_numerics::Vector2::one())?;
    backdrop_layer.SetRelativeSizeAdjustment(windows_numerics::Vector2::one())?;

    // VisualCollection ordering is bottom-to-top; sequential `InsertAtTop`
    // calls put each new layer above the previous one. Final stacking:
    // backdrop (bottom) < content (top).
    let root_children = root_visual.Children()?;
    root_children.InsertAtTop(&backdrop_layer)?;
    root_children.InsertAtTop(&content_layer)?;

    let backdrop_visual = window.compositor.CreateSpriteVisual()?;
    backdrop_visual.SetRelativeSizeAdjustment(windows_numerics::Vector2::one())?;
    backdrop_layer.Children()?.InsertAtBottom(&backdrop_visual)?;

    desktop_window_target.SetRoot(&root_visual)?;

    window.backdrop_tint.replace(Some(backdrop_visual));
    window.composition_target.replace(Some(desktop_window_target));
    window.composition_root.replace(Some(root_visual));
    window.backdrop_layer.replace(Some(backdrop_layer));
    window.content_layer.replace(Some(content_layer));
    Ok(())
}
