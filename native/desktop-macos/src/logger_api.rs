use crate::common::{ArraySize, BorrowedStrPtr, StrPtr};
use crate::logger::{append_exception_msg, clear_exception_msgs, exceptions_array, init_panic_handler, panic_payload_msg};
use log::{error, info};

#[repr(C)]
pub struct ExceptionsArray {
    pub items: *const StrPtr,
    pub count: ArraySize,
}

#[unsafe(no_mangle)]
pub extern "C" fn logger_check_exceptions() -> ExceptionsArray {
    let result = std::panic::catch_unwind(exceptions_array);
    result.unwrap_or_else(|payload| {
        let msg = panic_payload_msg(&payload);
        error!("logger_check_exceptions panic with: {msg}");
        ExceptionsArray {
            items: std::ptr::null(),
            count: 0,
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn logger_clear_exceptions() {
    let result = std::panic::catch_unwind(clear_exception_msgs);
    result.unwrap_or_else(|payload| {
        let msg = panic_payload_msg(&payload);
        error!("logger_clear_exceptions panic with: {msg}");
    });
}

#[allow(dead_code)]
#[repr(C)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[repr(C)]
pub struct LoggerConfiguration {
    pub file_path: BorrowedStrPtr,
    pub console_level: LogLevel,
    pub file_level: LogLevel,
}

#[unsafe(no_mangle)]
pub extern "C" fn logger_init(logger_configuration: &LoggerConfiguration) {
    let result = std::panic::catch_unwind(|| logger_configuration.init_logger());

    if let Err(payload) = result {
        let msg = panic_payload_msg(&payload);
        append_exception_msg(format!("logger_init panic with payload: {msg}"));
    }
    init_panic_handler();
    info!("Logger initialized");
}
