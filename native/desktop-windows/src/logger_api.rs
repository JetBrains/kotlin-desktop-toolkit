use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger_api::{ExceptionsArray, LoggerConfiguration},
};
use windows::{Win32::System::Diagnostics::Debug::OutputDebugStringA, core::PCSTR};

#[unsafe(no_mangle)]
pub extern "C" fn logger_check_exceptions() -> ExceptionsArray {
    desktop_common::logger_api::logger_check_exceptions_impl()
}

#[unsafe(no_mangle)]
pub extern "C" fn logger_clear_exceptions() {
    desktop_common::logger_api::logger_clear_exceptions_impl();
}

#[unsafe(no_mangle)]
pub extern "C" fn logger_init(logger_configuration: &LoggerConfiguration) {
    desktop_common::logger_api::logger_init_impl(logger_configuration);
}

#[unsafe(no_mangle)]
pub extern "C" fn logger_output_debug_string(message: BorrowedStrPtr) {
    let cstr = message
        .as_optional_cstr()
        .expect("Invalid debug string")
        .expect("The debug string pointer is null.");
    unsafe { OutputDebugStringA(PCSTR::from_raw(cstr.as_ptr().cast())) };
}
