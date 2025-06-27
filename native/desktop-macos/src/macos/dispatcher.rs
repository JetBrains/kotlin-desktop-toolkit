use std::ffi::c_void;

use desktop_common::logger::ffi_boundary;
use dispatch2::DispatchQueue;
use objc2::{MainThreadMarker, ffi};
use objc2_foundation::{NSQualityOfService, NSThread};

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

#[unsafe(no_mangle)]
pub extern "C" fn push_autorelease_pool() -> isize {
    ffi_boundary("push_autorelease_pool", || {
        let pool_ptr = unsafe { ffi::objc_autoreleasePoolPush() };
        Ok(pool_ptr as isize)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pop_autorelease_pool(pool_ptr: isize) {
    ffi_boundary("pop_autorelease_pool", || {
        unsafe { ffi::objc_autoreleasePoolPop(pool_ptr as *mut c_void) };
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn set_qos_for_current_thread(qos: isize) {
    ffi_boundary("set_qos_for_current_thread", || unsafe {
        let qos = NSQualityOfService(qos);
        if qos != NSQualityOfService::UserInteractive
            && qos != NSQualityOfService::UserInitiated
            && qos != NSQualityOfService::Utility
            && qos != NSQualityOfService::Background
            && qos != NSQualityOfService::Default
        {
            return Err(anyhow::anyhow!("Invalid QoS value: {}", qos.0));
        }
        NSThread::currentThread().setQualityOfService(qos);
        Ok(())
    });
}
