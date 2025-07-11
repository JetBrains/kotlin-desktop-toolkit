use std::{
    mem::ManuallyDrop,
    rc::{Rc, Weak},
};

use log::error;
use windows::{
    Win32::{
        Foundation::{COLORREF, HANDLE, HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Dwm::{
            DWM_SYSTEMBACKDROP_TYPE, DWMWA_CAPTION_COLOR, DWMWA_COLOR_NONE, DWMWA_SYSTEMBACKDROP_TYPE, DwmExtendFrameIntoClientArea,
            DwmSetWindowAttribute,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::MARGINS,
            HiDpi::GetDpiForWindow,
            WindowsAndMessaging::{
                CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow, GetPropW, RegisterClassExW, RemovePropW,
                SW_SHOW, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER, SetPropW, SetWindowPos, ShowWindow, USER_DEFAULT_SCREEN_DPI,
                WINDOW_EX_STYLE, WM_NCDESTROY, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
            },
        },
    },
    core::{PCWSTR, Result as WinResult, w},
};

use super::{
    application::Application,
    event_loop::EventLoop,
    geometry::{PhysicalPoint, PhysicalSize},
    window_api::{WindowId, WindowParams, WindowSystemBackdropType},
};

const WINDOW_PTR_PROP_NAME: PCWSTR = w!("KDT_WINDOW_PTR");

pub struct Window {
    hwnd: HWND,
    event_loop: Weak<EventLoop>,
    john_weak: Weak<Window>,
}

impl Window {
    pub fn new(params: &WindowParams, app: &Application) -> WinResult<Rc<Self>> {
        const WNDCLASS_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32WindowClass");
        let wndclass = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as _,
            lpszClassName: WNDCLASS_NAME,
            lpfnWndProc: Some(wndproc),
            style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
            ..Default::default()
        };
        let hwnd = unsafe {
            let _atom = RegisterClassExW(&wndclass);
            let instance = GetModuleHandleW(None)?;
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                WNDCLASS_NAME,
                w!("KotlinDesktopToolkit Win32 Window"),
                WS_OVERLAPPEDWINDOW,
                0, // CW_USEDEFAULT: i32 = -2147483648i32
                0, // CW_USEDEFAULT: i32 = -2147483648i32
                1,
                1,
                None,
                None,
                Some(instance.into()),
                None,
            )?
        };
        let window = Rc::new_cyclic(|weak| Self {
            hwnd,
            min_size: None,
            event_loop: Rc::downgrade(&app.event_loop()),
            john_weak: weak.clone(),
        });
        let scale = window.get_scale();
        let origin = PhysicalPoint::new(
            f32::round(params.origin.x.0 * scale + 0.5_f32) as i32,
            f32::round(params.origin.y.0 * scale + 0.5_f32) as i32,
        );
        let size = PhysicalSize::new(
            f32::round(params.size.width.0 * scale + 0.5_f32) as i32,
            f32::round(params.size.height.0 * scale + 0.5_f32) as i32,
        );
        window.set_position(origin, size)?;
        unsafe { SetPropW(hwnd, WINDOW_PTR_PROP_NAME, Some(HANDLE(window.john_weak.as_ptr() as _))) }?;
        Ok(window)
    }

    pub fn id(&self) -> WindowId {
        WindowId(self.hwnd.0 as isize)
    }

    #[inline]
    pub(crate) fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn get_scale(&self) -> f32 {
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        (dpi as f32) / (USER_DEFAULT_SCREEN_DPI as f32)
    }

    pub fn extend_content_into_titlebar(&self) -> WinResult<()> {
        let colorref = COLORREF(DWMWA_COLOR_NONE);
        let margins = MARGINS {
            cxLeftWidth: -1,
            cxRightWidth: -1,
            cyTopHeight: -1,
            cyBottomHeight: -1,
        };
        unsafe {
            // if we want to extend content into the titlebar area, it makes sense to remove any color from it
            DwmSetWindowAttribute(
                self.hwnd,
                DWMWA_CAPTION_COLOR,
                &raw const colorref as *const _,
                core::mem::size_of::<COLORREF>() as _,
            )?;
            DwmExtendFrameIntoClientArea(self.hwnd, &margins)
        }
    }

    pub fn apply_system_backdrop(&self, backdrop_type: WindowSystemBackdropType) -> WinResult<()> {
        let backdrop: DWM_SYSTEMBACKDROP_TYPE = backdrop_type.to_system();
        unsafe {
            DwmSetWindowAttribute(
                self.hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &raw const backdrop as *const _,
                core::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as _,
            )
        }
    }

    pub fn show(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_SHOW) };
    }

    pub fn set_position(&self, origin: PhysicalPoint, size: PhysicalSize) -> WinResult<()> {
        unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                origin.x.0,
                origin.y.0,
                size.width.0,
                size.height.0,
                SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_NOZORDER,
            )?
        };
        Ok(())
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        if let Err(err) = unsafe { DestroyWindow(self.hwnd) } {
            error!("Failed to destroy the window: {err:?}")
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if hwnd.0.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // WM_NCDESTROY is a special case: this is when we must clean up the extra resources used by the window
    if msg == WM_NCDESTROY {
        let _ = unsafe { RemovePropW(hwnd, WINDOW_PTR_PROP_NAME) };
        return LRESULT(0);
    }
    let raw = unsafe { GetPropW(hwnd, WINDOW_PTR_PROP_NAME).0 as *const Window };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // we reuse the weak reference on every iteration of the event loop, so we don't drop it here (see above)
    let this = ManuallyDrop::new(unsafe { Weak::from_raw(raw) });
    match this.upgrade() {
        Some(window) if hwnd == window.hwnd => {
            let event_loop = window.event_loop.upgrade().expect("event loop has been dropped");
            event_loop.window_proc(window.as_ref(), msg, wparam, lparam)
        }
        _ => {
            error!("could not upgrade the window weak reference, or the window pointer was incorrect");
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }
}
