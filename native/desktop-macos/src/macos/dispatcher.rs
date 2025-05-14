use desktop_common::logger::ffi_boundary;
use dispatch2::DispatchQueue;
use objc2::MainThreadMarker;

#[unsafe(no_mangle)]
pub extern "C" fn dispatcher_is_main_thread() -> bool {
    ffi_boundary("dispatcher_is_main_thread", || Ok(MainThreadMarker::new().is_some()))
}

#[unsafe(no_mangle)]
pub extern "C" fn dispatcher_main_exec_async(f: extern "C" fn()) {
    ffi_boundary("dispatcher_main_exec_async", || {
        #[allow(clippy::redundant_closure)]
        DispatchQueue::main().exec_async(move || f());
        Ok(())
    });
}
