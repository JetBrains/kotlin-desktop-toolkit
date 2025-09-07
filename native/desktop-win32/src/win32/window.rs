use std::{
    cell::RefCell,
    mem::ManuallyDrop,
    rc::{Rc, Weak},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::Context;
use windows::{
    UI::Composition::{Compositor, Desktop::DesktopWindowTarget, SpriteVisual},
    Win32::{
        Foundation::{COLORREF, ERROR_NO_UNICODE_TRANSLATION, HANDLE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::{
            Dwm::{
                DWM_SYSTEMBACKDROP_TYPE, DWMWA_CAPTION_COLOR, DWMWA_COLOR_NONE, DWMWA_SYSTEMBACKDROP_TYPE, DwmExtendFrameIntoClientArea,
                DwmSetWindowAttribute,
            },
            Gdi::{RDW_INVALIDATE, RDW_NOERASE, RDW_NOFRAME, RedrawWindow},
        },
        System::WinRT::Composition::ICompositorDesktopInterop,
        UI::{
            Controls::MARGINS,
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, GWL_STYLE, GetPropW, IDC_ARROW, LoadCursorW,
                RegisterClassExW, RemovePropW, SW_SHOW, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER, SetPropW, SetWindowLongPtrW,
                SetWindowPos, ShowWindow, USER_DEFAULT_SCREEN_DPI, WINDOW_STYLE, WM_NCCREATE, WM_NCDESTROY, WNDCLASSEXW,
                WS_EX_NOREDIRECTIONBITMAP,
            },
        },
    },
    core::{Error as WinError, HSTRING, Interface, PCWSTR, Result as WinResult, w},
};

use super::{
    event_loop::EventLoop,
    geometry::{LogicalPoint, LogicalSize, PhysicalPoint, PhysicalSize},
    utils,
    window_api::{WindowId, WindowParams, WindowStyle, WindowTitleBarKind},
};

/// cbindgen:ignore
const WINDOW_PTR_PROP_NAME: PCWSTR = w!("KDT_WINDOW_PTR");

pub struct Window {
    hwnd: RefCell<HWND>,
    compositor: Weak<Compositor>,
    composition_target: RefCell<Option<DesktopWindowTarget>>,
    sprite_visual: RefCell<Option<SpriteVisual>>,
    min_size: RefCell<Option<LogicalSize>>,
    origin: LogicalPoint,
    size: LogicalSize,
    style: WindowStyle,
    mouse_in_client: AtomicBool,
    event_loop: Weak<EventLoop>,
}

impl Window {
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(params: &WindowParams, event_loop: Weak<EventLoop>, compositor: Weak<Compositor>) -> WinResult<Rc<Self>> {
        const WNDCLASS_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32WindowClass");
        let instance = crate::get_dll_instance();
        let wndclass = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as _,
            hInstance: instance,
            lpszClassName: WNDCLASS_NAME,
            lpfnWndProc: Some(wndproc),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }?,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };
        let title = params
            .title
            .as_optional_str()
            .map_err(|_| WinError::from(ERROR_NO_UNICODE_TRANSLATION))?
            .map_or_else(HSTRING::new, HSTRING::from);
        let window = Rc::new(Self {
            hwnd: RefCell::new(HWND::default()),
            compositor,
            composition_target: RefCell::new(None),
            sprite_visual: RefCell::new(None),
            min_size: RefCell::new(None),
            origin: params.origin,
            size: params.size,
            style: params.style,
            mouse_in_client: AtomicBool::new(false),
            event_loop,
        });
        unsafe {
            let _atom = RegisterClassExW(&raw const wndclass);
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
                Some(Rc::downgrade(&window).into_raw().cast()),
            )?;
        }
        Ok(window)
    }

    #[must_use]
    pub fn id(&self) -> WindowId {
        WindowId(self.hwnd().0 as isize)
    }

    #[inline]
    pub(crate) fn hwnd(&self) -> HWND {
        *self.hwnd.borrow()
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

    #[must_use]
    pub const fn has_custom_title_bar(&self) -> bool {
        matches!(self.style.title_bar_kind, WindowTitleBarKind::Custom)
    }

    #[must_use]
    pub const fn is_resizable(&self) -> bool {
        self.style.is_resizable
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
                    core::mem::size_of::<COLORREF>() as _,
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
            let backdrop: DWM_SYSTEMBACKDROP_TYPE = self.style.system_backdrop_type.to_system();
            unsafe {
                DwmSetWindowAttribute(
                    self.hwnd(),
                    DWMWA_SYSTEMBACKDROP_TYPE,
                    (&raw const backdrop).cast(),
                    core::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as _,
                )?;
            }
        }
        Ok(())
    }

    pub fn show(&self) -> bool {
        unsafe { ShowWindow(self.hwnd(), SW_SHOW) }.as_bool()
    }

    pub fn set_position(&self, origin: PhysicalPoint, size: PhysicalSize) -> WinResult<()> {
        unsafe {
            SetWindowPos(
                self.hwnd(),
                None,
                origin.x.0,
                origin.y.0,
                size.width.0,
                size.height.0,
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
    pub(crate) fn is_mouse_in_client(&self) -> bool {
        self.mouse_in_client.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn set_is_mouse_in_client(&self, value: bool) {
        self.mouse_in_client.store(value, Ordering::Relaxed);
    }

    pub fn request_redraw(&self) -> WinResult<()> {
        unsafe { RedrawWindow(Some(self.hwnd()), None, None, RDW_INVALIDATE | RDW_NOFRAME | RDW_NOERASE) }.ok()
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
    window.hwnd.replace(hwnd);
    unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, window.style.to_system().0 as _) };
    let scale = window.get_scale();
    window.set_position(window.origin.to_physical(scale), window.size.to_physical(scale))?;
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
