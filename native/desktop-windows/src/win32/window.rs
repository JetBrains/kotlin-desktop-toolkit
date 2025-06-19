use std::{
    mem::ManuallyDrop,
    rc::{Rc, Weak},
};

use log::error;

use windows::{
    System::DispatcherQueueController,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, GetWindowLongPtrW, PostQuitMessage, RegisterClassExW, SW_SHOW, SetWindowLongPtrW, ShowWindow,
            WINDOW_EX_STYLE, WINDOW_LONG_PTR_INDEX, WM_CLOSE, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
        },
    },
    core::{PCWSTR, Result as WinResult, w},
};

use super::window_api::WindowParams;

pub struct Window<'a> {
    hwnd: HWND,
    dispatcher_queue_controller: &'a DispatcherQueueController,
    _john_weak: Weak<Window<'a>>,
}

impl<'a> Window<'a> {
    pub fn new(params: &WindowParams, dispatcher_queue_controller: &'a DispatcherQueueController) -> WinResult<Rc<Self>> {
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
        let window = Rc::new_cyclic(|weak| {
            unsafe { SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0), weak.as_ptr() as _) };
            Self {
                hwnd,
                dispatcher_queue_controller,
                _john_weak: weak.clone(),
            }
        });
        Ok(window)
    }

    pub fn show(&self) {
        let _ = unsafe { ShowWindow(self.hwnd, SW_SHOW) };
    }
}

impl Window<'_> {
    fn window_proc(&self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CLOSE => {
                if let Err(_err) = self.dispatcher_queue_controller.ShutdownQueueAsync() {
                    error!("failed to request the shutdown of the dispatcher queue");
                    unsafe { PostQuitMessage(0) };
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if hwnd.0.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    let raw = unsafe { GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(0)) as *const Window };
    if raw.is_null() {
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    }
    // we reuse the weak reference on every iteration of the event loop, so we shouldn't drop it
    let this = ManuallyDrop::new(unsafe { Weak::from_raw(raw) });
    match this.upgrade() {
        Some(window) if window.hwnd == hwnd => window.window_proc(hwnd, msg, wparam, lparam),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
