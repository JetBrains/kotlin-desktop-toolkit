use desktop_common::{ffi_utils::RustAllocatedRawPtr, logger::ffi_boundary};
use log::debug;

use windows::{Win32::Foundation::E_NOTIMPL, core::Error as WinError};

use super::application::Application;

pub type AppPtr<'a> = RustAllocatedRawPtr<'a>;

#[unsafe(no_mangle)]
pub extern "C" fn application_init() -> AppPtr<'static> {
    let app = ffi_boundary("application_init", || {
        debug!("Application init");
        Ok(Some(Application::new()?))
    });
    AppPtr::from_value(app)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_event_loop() {
    ffi_boundary("application_run_event_loop", || {
        debug!("Start event loop");
        anyhow::bail!(WinError::from_hresult(E_NOTIMPL))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_event_loop() {
    ffi_boundary("application_stop_event_loop", || {
        debug!("Stop event loop");
        anyhow::bail!(WinError::from_hresult(E_NOTIMPL))
    })
}
