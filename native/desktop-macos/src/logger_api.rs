use desktop_common::logger_api::{ExceptionsArray, LoggerConfiguration};

#[unsafe(no_mangle)]
pub extern "C" fn logger_check_exceptions() -> ExceptionsArray {
    desktop_common::logger_api::logger_check_exceptions_impl()
}

#[unsafe(no_mangle)]
pub extern "C" fn logger_clear_exceptions() {
    desktop_common::logger_api::logger_clear_exceptions_impl()
}

#[unsafe(no_mangle)]
pub extern "C" fn logger_init(logger_configuration: &LoggerConfiguration) {
    desktop_common::logger_api::logger_init_impl(logger_configuration);
}
