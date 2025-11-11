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
    UI::Composition::{Compositor, Desktop::DesktopWindowTarget, SpriteVisual},
    Win32::{
        Foundation::{COLORREF, HANDLE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Dwm::{
                DWM_SYSTEMBACKDROP_TYPE, DWMWA_CAPTION_COLOR, DWMWA_COLOR_NONE, DWMWA_SYSTEMBACKDROP_TYPE, DwmExtendFrameIntoClientArea,
                DwmSetWindowAttribute,
            },
            Gdi::{MONITOR_DEFAULTTONEAREST, MonitorFromWindow, RDW_INVALIDATE, RDW_NOERASE, RDW_NOFRAME, RedrawWindow},
        },
        System::WinRT::Composition::ICompositorDesktopInterop,
        UI::{
            Controls::MARGINS,
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, GWL_STYLE, GetPropW, IDC_ARROW, LoadCursorW,
                PostMessageW, RegisterClassExW, RemovePropW, SW_SHOW, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER, SetPropW,
                SetWindowLongPtrW, SetWindowPos, ShowWindow, USER_DEFAULT_SCREEN_DPI, WINDOW_STYLE, WM_CLOSE, WM_NCCREATE, WM_NCDESTROY,
                WNDCLASSEXW, WS_EX_NOREDIRECTIONBITMAP,
            },
        },
    },
    core::{Interface, PCWSTR, Result as WinResult, w},
};

use super::{
    event_loop::EventLoop,
    geometry::{LogicalPoint, LogicalSize},
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

pub struct Window {
    id: WindowId,
    hwnd: AtomicPtr<core::ffi::c_void>,
    compositor: Weak<Compositor>,
    composition_target: RefCell<Option<DesktopWindowTarget>>,
    sprite_visual: RefCell<Option<SpriteVisual>>,
    min_size: RefCell<Option<LogicalSize>>,
    origin: RefCell<LogicalPoint>,
    size: RefCell<LogicalSize>,
    style: RefCell<WindowStyle>,
    pointer_in_client: AtomicBool,
    event_loop: Weak<EventLoop>,
}

impl Window {
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(window_id: WindowId, event_loop: Weak<EventLoop>, compositor: Weak<Compositor>) -> WinResult<Self> {
        static WNDCLASS_INIT: OnceLock<u16> = OnceLock::new();
        if WNDCLASS_INIT.get().is_none() {
            let wndclass = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as _,
                hInstance: crate::get_dll_instance(),
                lpszClassName: WNDCLASS_NAME,
                lpfnWndProc: Some(wndproc),
                hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
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
            sprite_visual: RefCell::new(None),
            min_size: RefCell::new(None),
            origin: RefCell::default(),
            size: RefCell::default(),
            style: RefCell::default(),
            pointer_in_client: AtomicBool::new(false),
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
    pub(crate) fn get_visual(&self) -> anyhow::Result<SpriteVisual> {
        self.sprite_visual
            .borrow()
            .as_ref()
            .context("Window has not been created yet")
            .cloned()
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
    pub fn is_resizable(&self) -> bool {
        self.style.borrow().is_resizable
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn extend_content_into_titlebar(&self) -> WinResult<()> {
        if utils::is_windows_11_build_22000_or_higher() {
            let colorref = COLORREF(DWMWA_COLOR_NONE);
            unsafe {
                // if we want to extend content into the titlebar area, it makes sense to remove any color from it
                DwmSetWindowAttribute(
                    self.hwnd(),
                    DWMWA_CAPTION_COLOR,
                    (&raw const colorref).cast(),
                    size_of::<COLORREF>() as _,
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

    #[allow(clippy::cast_possible_truncation)]
    pub fn apply_system_backdrop(&self) -> WinResult<()> {
        if utils::is_windows_11_build_22621_or_higher() {
            let backdrop: DWM_SYSTEMBACKDROP_TYPE = self.style.borrow().system_backdrop_type.to_system();
            unsafe {
                DwmSetWindowAttribute(
                    self.hwnd(),
                    DWMWA_SYSTEMBACKDROP_TYPE,
                    (&raw const backdrop).cast(),
                    size_of::<DWM_SYSTEMBACKDROP_TYPE>() as _,
                )?;
            }
        }
        Ok(())
    }

    pub fn show(&self) -> bool {
        unsafe { ShowWindow(self.hwnd(), SW_SHOW) }.as_bool()
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

    #[must_use]
    pub fn get_min_size(&self) -> Option<LogicalSize> {
        *self.min_size.borrow()
    }

    pub fn set_min_size(&self, size: LogicalSize) {
        self.min_size.replace(Some(size));
    }

    #[inline]
    pub(crate) fn is_pointer_in_client(&self) -> bool {
        self.pointer_in_client.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn set_is_pointer_in_client(&self, value: bool) {
        self.pointer_in_client.store(value, Ordering::Relaxed);
    }

    pub fn request_redraw(&self) -> WinResult<()> {
        unsafe { RedrawWindow(Some(self.hwnd()), None, None, RDW_INVALIDATE | RDW_NOFRAME | RDW_NOERASE) }.ok()
    }

    pub fn request_close(&self) -> WinResult<()> {
        unsafe { PostMessageW(Some(self.hwnd()), WM_CLOSE, WPARAM::default(), LPARAM::default()) }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        log::debug!("window drop");
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
            let _ = unsafe { Rc::from_raw(raw.0.cast::<Window>()) };
        }
        return LRESULT(0);
    }

    let raw = unsafe { GetPropW(hwnd, WINDOW_PTR_PROP_NAME).0.cast::<Window>() };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }

    // we reuse the weak reference on every iteration of the event loop, so we don't drop it here (see above)
    let window = ManuallyDrop::new(unsafe { Rc::from_raw(raw) });
    if hwnd == window.hwnd() {
        let event_loop = window.event_loop.upgrade().expect("event loop has been dropped");
        event_loop.window_proc(Rc::as_ref(&window), msg, wparam, lparam)
    } else {
        log::error!("the window pointer was incorrect");
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }
}

fn on_nccreate(hwnd: HWND, lparam: LPARAM) -> anyhow::Result<()> {
    let create_struct = unsafe { (lparam.0 as *mut CREATESTRUCTW).as_mut() }.context("CREATESTRUCTW is null")?;

    let window_ptr = create_struct.lpCreateParams.cast_const().cast::<Window>();
    let john_weak = ManuallyDrop::new(unsafe { Weak::from_raw(window_ptr) });

    let window = john_weak.upgrade().context("failed to upgrade the window weak reference")?;
    initialize_window(&window, hwnd).context("failed to initialize the window")?;

    unsafe { SetPropW(hwnd, WINDOW_PTR_PROP_NAME, Some(HANDLE(Rc::into_raw(window).cast_mut().cast()))) }
        .context("failed to set the window property")
}

fn initialize_window(window: &Window, hwnd: HWND) -> anyhow::Result<()> {
    window.hwnd.store(hwnd.0, Ordering::Release);
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, window.style.borrow().to_system().0 as _) };
    window.set_position(*window.origin.borrow(), *window.size.borrow())?;
    initialize_composition(window, hwnd).context("failed to initialize composition")
}

fn initialize_composition(window: &Window, hwnd: HWND) -> anyhow::Result<()> {
    let compositor = window
        .compositor
        .upgrade()
        .context("failed to upgrade the compositor weak reference")?;
    let compositor_interop: ICompositorDesktopInterop = compositor.cast()?;
    let desktop_window_target = unsafe { compositor_interop.CreateDesktopWindowTarget(hwnd, true) }?;
    let sprite_visual = compositor.CreateSpriteVisual()?;
    desktop_window_target.SetRoot(&sprite_visual)?;
    window.composition_target.replace(Some(desktop_window_target));
    window.sprite_visual.replace(Some(sprite_visual));
    Ok(())
}
