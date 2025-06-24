use desktop_common::{ffi_utils::RustAllocatedRawPtr, logger::ffi_boundary};
use log::debug;

use super::{application::Application, events::EventHandler};

pub type AppPtr<'a> = RustAllocatedRawPtr<'a>;

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    pub event_handler: EventHandler,
}

#[unsafe(no_mangle)]
pub extern "C" fn application_init(callbacks: ApplicationCallbacks) -> AppPtr<'static> {
    let app = ffi_boundary("application_init", || {
        debug!("Application init");
        Ok(Some(Application::new(callbacks.event_handler)?))
    });
    AppPtr::from_value(app)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_event_loop(app_ptr: AppPtr) {
    ffi_boundary("application_run_event_loop", || {
        debug!("Start event loop");

        let app = unsafe { app_ptr.borrow::<Application>() };
        app.event_loop().run();

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_event_loop(app_ptr: AppPtr) {
    ffi_boundary("application_stop_event_loop", || {
        debug!("Stop event loop");

        let app = unsafe { app_ptr.borrow::<Application>() };
        app.event_loop().shutdown()?;

        Ok(())
    })
}
