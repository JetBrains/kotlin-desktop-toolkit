use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedRawPtr},
    logger::ffi_boundary,
};
use log::debug;

use super::{
    application::Application,
    application_api::AppPtr,
    geometry::{PhysicalPoint, PhysicalSize},
    window::Window,
};

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
        let window = Window::new(&params, app.dispatcher_queue_controller())?;

        Ok(Some(window))
    });
    WindowPtr::from_rc(window)
}

#[unsafe(no_mangle)]
pub extern "C" fn window_drop(window_ptr: WindowPtr) {
    ffi_boundary("window_drop", || {
        let _window = unsafe { window_ptr.to_owned::<Window>() };
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show(window_ptr: WindowPtr) {
    ffi_boundary("window_show", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.show();
        Ok(())
    });
}
