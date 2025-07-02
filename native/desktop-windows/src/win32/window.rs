use std::{
    mem::ManuallyDrop,
    rc::{Rc, Weak},
};

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
            WindowsAndMessaging::{
                CS_HREDRAW, CS_OWNDC, CS_VREDRAW, CreateWindowExW, DefWindowProcW, GetPropW, RegisterClassExW, RemovePropW, SW_SHOW,
                SetPropW, ShowWindow, WINDOW_EX_STYLE, WM_NCDESTROY, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
            },
        },
    },
    core::{PCWSTR, Result as WinResult, w},
};

use super::{
    application::Application,
    event_loop::EventLoop,
    window_api::{WindowId, WindowParams, WindowSystemBackdropType},
};

const WINDOW_EVENT_LOOP_PROP_NAME: PCWSTR = w!("KOTLIN_DESKTOP_TOOLKIT_EVENT_LOOP_PTR");

pub struct Window {
    hwnd: HWND,
    _john_weak: Weak<EventLoop>,
}

impl Window {
    pub fn new(params: &WindowParams, app: &Application) -> WinResult<Self> {
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
                params.origin.x.0, // CW_USEDEFAULT: i32 = -2147483648i32
                params.origin.y.0, // CW_USEDEFAULT: i32 = -2147483648i32
                params.size.width.0,
                params.size.height.0,
                None,
                None,
                Some(instance.into()),
                None,
            )?
        };
        let event_loop = Rc::downgrade(&app.event_loop());
        unsafe { SetPropW(hwnd, WINDOW_EVENT_LOOP_PROP_NAME, Some(HANDLE(event_loop.as_ptr() as _))) }?;
        Ok(Self {
            hwnd,
            _john_weak: event_loop,
        })
    }

    pub fn id(&self) -> WindowId {
        WindowId(self.hwnd.0 as isize)
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
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if hwnd.0.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // WM_NCDESTROY is a special case: this is when we must clean up the extra resources used by the window
    if msg == WM_NCDESTROY {
        let raw = unsafe { RemovePropW(hwnd, WINDOW_EVENT_LOOP_PROP_NAME) }
            .unwrap_or(HANDLE::default())
            .0 as *const EventLoop;
        if !raw.is_null() {
            // this is the moment when we can drop the weak reference
            let _ = unsafe { Weak::from_raw(raw) };
        }
        return LRESULT(0);
    }
    let raw = unsafe { GetPropW(hwnd, WINDOW_EVENT_LOOP_PROP_NAME).0 as *const EventLoop };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // we reuse the weak reference on every iteration of the event loop, so we don't drop it here (see above)
    let this = ManuallyDrop::new(unsafe { Weak::from_raw(raw) });
    match this.upgrade() {
        Some(app) => app.window_proc(hwnd, msg, wparam, lparam),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
