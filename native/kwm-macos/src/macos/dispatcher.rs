use crate::logger::ffi_boundary;

#[no_mangle]
pub extern "C" fn dispatcher_is_main_thread() -> bool {
    ffi_boundary("dispatcher_is_main_thread", || {
        Ok(objc2_foundation::is_main_thread())
    })
}

#[no_mangle]
pub extern "C" fn dispatcher_main_exec_async(f: extern "C" fn()) {
    ffi_boundary("dispatcher_main_exec_async", || {
        dispatch::Queue::main().exec_async(move || {
            f()
        });
        Ok(())
    });
}