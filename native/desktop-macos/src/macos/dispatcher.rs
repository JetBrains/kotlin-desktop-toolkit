use std::ffi::c_void;

use block2::RcBlock;
use desktop_common::logger::ffi_boundary;
use dispatch2::DispatchQueue;
use objc2::{MainThreadMarker, ffi, sel};
use objc2_foundation::{NSBlockOperation, NSObjectNSThreadPerformAdditions, NSQualityOfService, NSThread};

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

fn run_block_on_main_thread_sync<F: Fn() + 'static>(f: F) {
    let block = RcBlock::new(move || {
        f();
    });
    let op = unsafe { NSBlockOperation::blockOperationWithBlock(&block) };
    unsafe { op.performSelectorOnMainThread_withObject_waitUntilDone(sel!(start), None, true) };
}

#[unsafe(no_mangle)]
pub extern "C" fn dispatcher_start_on_main_thread(f: extern "C" fn()) {
    ffi_boundary("dispatcher_start_on_main_thread", || {
        #[allow(clippy::redundant_closure)]
        run_block_on_main_thread_sync(move || f());
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
    ffi_boundary("set_qos_for_current_thread", || {
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
