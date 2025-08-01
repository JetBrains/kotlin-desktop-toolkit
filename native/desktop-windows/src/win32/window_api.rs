use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedRawPtr},
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;
use windows::Win32::Foundation::{HWND, INVALID_HANDLE_VALUE};

use super::{
    application::Application,
    application_api::AppPtr,
    geometry::{PhysicalPoint, PhysicalSize},
    window::Window,
};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WindowId(pub isize);

impl PanicDefault for WindowId {
    fn default() -> Self {
        WindowId(INVALID_HANDLE_VALUE.0 as isize)
    }
}

impl From<WindowId> for HWND {
    fn from(value: WindowId) -> Self {
        HWND(value.0 as _)
    }
}

impl From<HWND> for WindowId {
    fn from(value: HWND) -> Self {
        WindowId(value.0 as _)
    }
}

pub type WindowPtr<'a> = RustAllocatedRawPtr<'a>;

#[repr(C)]
pub struct WindowParams<'a> {
    pub origin: PhysicalPoint,
    pub size: PhysicalSize,
    pub title: BorrowedStrPtr<'a>,

    pub is_resizable: bool,
    pub is_closable: bool,
    pub is_minimizable: bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn window_create(app_ptr: AppPtr, params: WindowParams) -> WindowPtr<'static> {
    let window = ffi_boundary("window_create", || {
        debug!("window_create");

        let app = unsafe { app_ptr.borrow::<Application>() };
        let window = Window::new(&params, app)?;

        Ok(Some(window))
    });
    WindowPtr::from_value(window)
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_window_id(window_ptr: WindowPtr) -> WindowId {
    ffi_boundary("window_get_window_id", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.id())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show(window_ptr: WindowPtr) {
    ffi_boundary("window_show", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.show();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_drop(window_ptr: WindowPtr) {
    ffi_boundary("window_drop", || {
        let _window = unsafe { window_ptr.to_owned::<Window>() };
        Ok(())
    });
}
