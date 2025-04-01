use std::ffi::{CStr, c_void};

use super::{application::Application, application_state::EglInstance, xdg_desktop_settings_api::XdgDesktopSetting};
use desktop_common::ffi_utils::RustAllocatedRawPtr;
use desktop_common::logger::ffi_boundary;
use log::debug;

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    // Returns true if application should terminate, otherwise termination will be canceled
    pub on_should_terminate: extern "C" fn() -> bool,
    pub on_will_terminate: extern "C" fn(),
    pub on_display_configuration_change: extern "C" fn(),
    pub on_xdg_desktop_settings_change: extern "C" fn(XdgDesktopSetting),
}

pub type AppPtr<'a> = RustAllocatedRawPtr<'a>;

#[unsafe(no_mangle)]
pub extern "C" fn application_init(callbacks: ApplicationCallbacks) -> AppPtr<'static> {
    let app = ffi_boundary("application_init", || {
        debug!("Application Init");
        Ok(Some(Application::new(callbacks)?))
    });
    AppPtr::from_value(app)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_event_loop(mut app_ptr: AppPtr) {
    ffi_boundary("application_run_event_loop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.run()
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_event_loop(mut app_ptr: AppPtr) {
    ffi_boundary("application_stop_event_loop", || {
        debug!("Stop event loop");
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.exit = true;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_shutdown(app_ptr: AppPtr) {
    ffi_boundary("application_shutdown", || {
        let mut app = unsafe { app_ptr.to_owned::<Application>() };
        app.exit = true;
        Ok(())
    });
}

#[repr(C)]
pub struct GetEglProcFuncData {
    pub f: extern "C" fn(ctx: *const c_void, name: *const std::ffi::c_char) -> *const c_void,
    pub ctx: *const c_void,
}

extern "C" fn egl_get_gl_proc(ctx_ptr: *const c_void, name_ptr: *const std::ffi::c_char) -> *const c_void {
    let name_cstr = unsafe { CStr::from_ptr(name_ptr) };
    let name = name_cstr.to_str().unwrap();
    debug!("egl_get_gl_proc for {name}");
    let egl_ptr = ctx_ptr.cast::<EglInstance>();
    let egl = unsafe { egl_ptr.as_ref() }.unwrap();
    if let Some(f) = egl.get_proc_address(name) {
        f as *const c_void
    } else {
        debug!("egl_get_gl_proc end for {name} returning null");
        std::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_egl_proc_func(app_ptr: AppPtr) -> GetEglProcFuncData {
    debug!("application_get_egl_proc_func");
    let app = unsafe { app_ptr.borrow::<Application>() };
    let ctx_ptr: *const EglInstance = if let Some(r) = app.state.egl.as_ref() {
        r
    } else {
        std::ptr::null()
    };
    GetEglProcFuncData {
        f: egl_get_gl_proc,
        ctx: ctx_ptr.cast(),
    }
}
