use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedRawPtr},
    logger::ffi_boundary,
};
use windows::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};

use super::{application::Application, events::EventHandler, strings::copy_from_utf8_string};

pub type AppPtr<'a> = RustAllocatedRawPtr<'a>;

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    pub event_handler: EventHandler,
}

#[unsafe(no_mangle)]
pub extern "C" fn application_init_apartment() {
    ffi_boundary("application_init_apartment", || {
        Application::init_apartment()?;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_init(callbacks: ApplicationCallbacks) -> AppPtr<'static> {
    let app = ffi_boundary("application_init", || Ok(Some(Application::new(callbacks.event_handler)?)));
    AppPtr::from_value(app)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_is_dispatcher_thread(app_ptr: AppPtr) -> bool {
    ffi_boundary("application_is_dispatcher_thread", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        Ok(app.is_dispatcher_thread()?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_dispatcher_invoke(app_ptr: AppPtr, callback: extern "C" fn()) -> bool {
    ffi_boundary("application_dispatcher_invoke", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        Ok(app.invoke_on_dispatcher_queue(callback)?)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_event_loop(app_ptr: AppPtr) {
    ffi_boundary("application_run_event_loop", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.run_event_loop();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_event_loop(app_ptr: AppPtr) {
    ffi_boundary("application_stop_event_loop", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.shutdown()?;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_open_url(url: BorrowedStrPtr) {
    ffi_boundary("application_open_url", || {
        let url = copy_from_utf8_string(&url)?;
        let result = unsafe { ShellExecuteW(None, windows::core::w!("open"), &url, None, None, SW_SHOWNORMAL) };
        // https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew
        if result.0 as isize <= 32 {
            return Err(windows::core::Error::from_thread().into());
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_drop(app_ptr: AppPtr) {
    ffi_boundary("application_drop", || {
        let _application = unsafe { app_ptr.to_owned::<Application>() };
        Ok(())
    });
}
