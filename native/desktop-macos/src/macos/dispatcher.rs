use objc2::MainThreadMarker;

use crate::logger::ffi_boundary;

#[no_mangle]
pub extern "C" fn dispatcher_is_main_thread() -> bool {
    ffi_boundary("dispatcher_is_main_thread", || Ok(MainThreadMarker::new().is_some()))
}

#[no_mangle]
pub extern "C" fn dispatcher_main_exec_async(f: extern "C" fn()) {
    ffi_boundary("dispatcher_main_exec_async", || {
        #[allow(clippy::redundant_closure)]
        dispatch2::Queue::main().exec_async(move || f());
        Ok(())
    });
}
