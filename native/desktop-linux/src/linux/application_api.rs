use super::{application::Application, application_state::EglInstance, xdg_desktop_settings_api::XdgDesktopSetting};
use anyhow::bail;
use desktop_common::ffi_utils::{BorrowedOpaquePtr, BorrowedStrPtr, RustAllocatedRawPtr};
use desktop_common::logger::ffi_boundary;
use log::debug;

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    pub on_application_started: extern "C" fn(),
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

#[derive(Debug)]
#[repr(C)]
pub struct GetEglProcFuncData<'a> {
    pub f: extern "C" fn(ctx: BorrowedOpaquePtr<'a>, name: BorrowedStrPtr) -> Option<extern "system" fn()>,
    pub ctx: BorrowedOpaquePtr<'a>,
}

extern "C" fn egl_get_proc_address(ctx_ptr: BorrowedOpaquePtr<'_>, name_ptr: BorrowedStrPtr) -> Option<extern "system" fn()> {
    let name = name_ptr.as_str().unwrap();
    debug!("egl_get_gl_proc for {name}");
    let egl = unsafe { ctx_ptr.borrow::<EglInstance>() }.unwrap();
    egl.get_proc_address(name)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_egl_proc_func(app_ptr: AppPtr<'_>) -> GetEglProcFuncData<'_> {
    debug!("application_get_egl_proc_func");
    let app = unsafe { app_ptr.borrow::<Application>() };
    GetEglProcFuncData {
        f: egl_get_proc_address,
        ctx: BorrowedOpaquePtr::new(app.state.egl.as_ref()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn application_is_event_loop_thread(app_ptr: AppPtr<'_>) -> bool {
    ffi_boundary("application_is_event_loop_thread", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(t) = app.state.event_loop_thread_id {
            let current_thread_id = std::thread::current().id();
            Ok(t == current_thread_id)
        } else {
            bail!("Event loop not yet started")
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_on_event_loop_async(app_ptr: AppPtr<'_>, f: extern "C" fn()) {
    ffi_boundary("application_run_on_event_loop_async", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(s) = &app.state.run_on_event_loop {
            s.send(f).map_err(std::convert::Into::into)
        } else {
            bail!("Event loop not yet started")
        }
    });
}
