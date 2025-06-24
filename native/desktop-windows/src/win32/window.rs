use std::{
    mem::ManuallyDrop,
    rc::{Rc, Weak},
};

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, GetWindowLongPtrW, RegisterClassExW, SW_SHOW, SetWindowLongPtrW, ShowWindow, WINDOW_EX_STYLE,
            WINDOW_LONG_PTR_INDEX, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
        },
    },
    core::{PCWSTR, Result as WinResult, w},
};

use super::{
    application::Application,
    event_loop::EventLoop,
    window_api::{WindowId, WindowParams},
};

pub struct Window {
    hwnd: HWND,
    _john_weak: Weak<EventLoop>,
}

impl Window {
    pub fn new(params: &WindowParams, app: &Application) -> WinResult<Self> {
        const WNDCLASS_NAME: PCWSTR = w!("KotlinDesktopToolkitWin32WindowClass");
        let wndclass = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as _,
            cbWndExtra: size_of::<*const Window>() as _,
            lpszClassName: WNDCLASS_NAME,
            lpfnWndProc: Some(wndproc),
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
        unsafe { SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), event_loop.as_ptr() as _) };
        Ok(Self {
            hwnd,
            _john_weak: event_loop,
        })
    }

    pub fn id(&self) -> WindowId {
        WindowId(self.hwnd.0 as isize)
    }

    pub fn show(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_SHOW) };
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if hwnd.0.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let raw = unsafe { GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0)) as *const EventLoop };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // we reuse the weak reference on every iteration of the event loop, so we shouldn't drop it
    let this = ManuallyDrop::new(unsafe { Weak::from_raw(raw) });
    match this.upgrade() {
        Some(app) => app.window_proc(hwnd, msg, wparam, lparam),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
