use std::{
    cell::RefCell,
    mem::ManuallyDrop,
    rc::{Rc, Weak},
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicPtr, Ordering},
    },
};

use anyhow::Context;
use windows::{
    UI::Composition::{Compositor, ContainerVisual, Desktop::DesktopWindowTarget, SpriteVisual},
    Win32::{
        Foundation::{COLORREF, HANDLE, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::{
            Dwm::{
                DWM_SYSTEMBACKDROP_TYPE, DWMWA_CAPTION_COLOR, DWMWA_COLOR_NONE, DWMWA_EXTENDED_FRAME_BOUNDS, DWMWA_SYSTEMBACKDROP_TYPE,
                DWMWA_USE_IMMERSIVE_DARK_MODE, DwmExtendFrameIntoClientArea, DwmGetWindowAttribute, DwmSetWindowAttribute,
            },
            Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow, RDW_INVALIDATE, RDW_NOERASE, RDW_NOFRAME, RedrawWindow},
        },
        System::WinRT::Composition::ICompositorDesktopInterop,
        UI::{
            Controls::MARGINS,
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, GWL_STYLE, GetClientRect, GetPropW,
                IsIconic, IsZoomed, PostMessageW, RegisterClassExW, RemovePropW, SW_SHOW, SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED,
                SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER, SetCursor, SetPropW, SetWindowLongPtrW, SetWindowPos, SetWindowTextW,
                ShowWindow, USER_DEFAULT_SCREEN_DPI, WINDOW_STYLE, WM_CLOSE, WM_NCCREATE, WM_NCDESTROY, WNDCLASSEXW,
                WS_EX_NOREDIRECTIONBITMAP,
            },
        },
    },
    core::{HSTRING, Interface, PCWSTR, Result as WinResult, w},
};

use super::{
    cursor::{Cursor, CursorIcon},
    event_loop::EventLoop,
    geometry::{LogicalPoint, LogicalRect, LogicalSize},
    pointer::PointerClickCounter,
    screen::{self, ScreenInfo},
    strings::copy_from_utf8_string,
    utils,
    window_api::{WindowParams, WindowStyle, WindowTitleBarKind},
};

/// cbindgen:ignore
const WINDOW_PTR_PROP_NAME: PCWSTR = w!("KDT_WINDOW_PTR");
/// cbindgen:ignore
const WNDCLASS_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32WindowClass");

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WindowId(pub isize);

#[allow(clippy::struct_field_names)]
pub struct Window {
    id: WindowId,
    hwnd: AtomicPtr<core::ffi::c_void>,
    compositor: Compositor,
    composition_target: RefCell<Option<DesktopWindowTarget>>,
    composition_root: RefCell<Option<ContainerVisual>>,
    min_size: RefCell<Option<LogicalSize>>,
    origin: RefCell<LogicalPoint>,
    size: RefCell<LogicalSize>,
    style: RefCell<WindowStyle>,
    pointer_in_window: AtomicBool,
    pointer_click_counter: RefCell<PointerClickCounter>,
    cursor: RefCell<Option<Cursor>>,
    event_loop: Weak<EventLoop>,
}

impl Window {
    pub fn new(window_id: WindowId, event_loop: Weak<EventLoop>, compositor: Compositor) -> WinResult<Self> {
        static WNDCLASS_INIT: OnceLock<u16> = OnceLock::new();
        if WNDCLASS_INIT.get().is_none() {
            let wndclass = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>().try_into()?,
                hInstance: crate::get_dll_instance(),
                lpszClassName: WNDCLASS_NAME,
                lpfnWndProc: Some(wndproc),
                style: CS_HREDRAW | CS_VREDRAW,
                ..Default::default()
            };
            let atom = unsafe { RegisterClassExW(&raw const wndclass) };
            WNDCLASS_INIT.get_or_init(|| atom);
        }
        let window = Self {
            id: window_id,
            hwnd: AtomicPtr::default(),
            compositor,
            composition_target: RefCell::new(None),
            composition_root: RefCell::new(None),
            min_size: RefCell::new(None),
            origin: RefCell::default(),
            size: RefCell::new(LogicalSize::new(0.0, 0.0)),
            style: RefCell::default(),
            pointer_in_window: AtomicBool::new(false),
            pointer_click_counter: RefCell::new(PointerClickCounter::new()),
            cursor: RefCell::new(None),
            event_loop,
        };
        Ok(window)
    }

    pub fn create(window: &Rc<Self>, creation_params: &WindowParams) -> WinResult<()> {
        let instance = crate::get_dll_instance();
        window.origin.replace(creation_params.origin);
        window.size.replace(creation_params.size);
        window.style.replace(creation_params.style);
        let title = copy_from_utf8_string(&creation_params.title)?;
        unsafe {
            CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP,
                WNDCLASS_NAME,
                &title,
                WINDOW_STYLE(0),
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
        self.composition_root
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

    pub fn get_rect(&self) -> anyhow::Result<LogicalRect> {
        let mut rect = RECT::default();
        unsafe {
            DwmGetWindowAttribute(
                self.hwnd(),
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&raw mut rect).cast(),
                size_of::<RECT>().try_into()?,
            )?;
        };
        let scale = self.get_scale();
        let origin = LogicalPoint::from_physical(rect.left, rect.top, scale);
        let size = LogicalSize::from_physical(rect.right - rect.left, rect.bottom - rect.top, scale);
        Ok(LogicalRect { origin, size })
    }

    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn get_scale(&self) -> f32 {
        let dpi = unsafe { GetDpiForWindow(self.hwnd()) };
        (dpi as f32) / (USER_DEFAULT_SCREEN_DPI as f32)
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

    pub fn maximize(&self) -> bool {
        unsafe { ShowWindow(self.hwnd(), SW_SHOWMAXIMIZED) }.as_bool()
    }

    pub fn minimize(&self) -> bool {
        unsafe { ShowWindow(self.hwnd(), SW_SHOWMINIMIZED) }.as_bool()
    }

    pub fn show(&self) -> bool {
        unsafe { ShowWindow(self.hwnd(), SW_SHOW) }.as_bool()
    }

    pub fn set_cursor(&self, cursor: Cursor) {
        unsafe { SetCursor(Some(cursor.as_native())) };
        self.cursor.replace(Some(cursor));
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
        if utils::is_windows_11_build_22000_or_higher() {
            let enablement = if enabled {
                windows::Win32::Foundation::TRUE
            } else {
                windows::Win32::Foundation::FALSE
            };
            unsafe {
                DwmSetWindowAttribute(
                    self.hwnd(),
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    (&raw const enablement).cast(),
                    size_of::<windows::core::BOOL>().try_into()?,
                )?;
            }
        }
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

    #[must_use]
    pub fn get_min_size(&self) -> Option<LogicalSize> {
        *self.min_size.borrow()
    }

    pub fn set_min_size(&self, size: LogicalSize) {
        self.min_size.replace(Some(size));
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
            let _ = unsafe { Weak::from_raw(raw.0.cast::<Window>()) };
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
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, window.style.borrow().to_system().0 as _) };
    window.set_position(*window.origin.borrow(), *window.size.borrow())?;
    initialize_composition(window, hwnd).context("failed to initialize composition")?;
    window.set_cursor(Cursor::load_from_system(CursorIcon::Arrow)?);
    Ok(())
}

fn initialize_composition(window: &Window, hwnd: HWND) -> anyhow::Result<()> {
    let compositor_interop: ICompositorDesktopInterop = window.compositor.cast()?;
    let desktop_window_target = unsafe { compositor_interop.CreateDesktopWindowTarget(hwnd, true) }?;
    let root_visual = window.compositor.CreateContainerVisual()?;
    desktop_window_target.SetRoot(&root_visual)?;
    window.composition_target.replace(Some(desktop_window_target));
    window.composition_root.replace(Some(root_visual));
    Ok(())
}
