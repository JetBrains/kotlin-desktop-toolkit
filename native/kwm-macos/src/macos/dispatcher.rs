

#[no_mangle]
pub extern "C" fn dispatcher_is_main_thread() -> bool {
    return objc2_foundation::is_main_thread();
}

#[no_mangle]
pub extern "C" fn dispatcher_main_exec_async(f: extern "C" fn()) {
    dispatch::Queue::main().exec_async(move || {
        f()
    });
}