use desktop_common::logger::ffi_boundary;
use objc2::MainThreadMarker;
use objc2_foundation::{NSQualityOfService, NSThread};

#[unsafe(no_mangle)]
pub extern "C" fn dispatcher_is_main_thread() -> bool {
    ffi_boundary("dispatcher_is_main_thread", || Ok(MainThreadMarker::new().is_some()))
}

#[unsafe(no_mangle)]
pub extern "C" fn dispatcher_main_exec_async(f: extern "C" fn()) {
    ffi_boundary("dispatcher_main_exec_async", || {
        #[allow(clippy::redundant_closure)]
        dispatch2::Queue::main().exec_async(move || f());
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
