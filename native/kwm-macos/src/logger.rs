use core::panic;
use std::{any::Any, borrow::{Borrow, BorrowMut}, cell::{Cell, RefCell}, ffi::{CStr, CString}, fmt::format, panic::AssertUnwindSafe};

use anyhow::Context;
use log::{debug, error, info, trace, warn, Level, LevelFilter, Metadata, Record, SetLoggerError};
use log4rs::{append::{console::{ConsoleAppender, Target}, file::FileAppender}, config::{Appender, Root}, encode::pattern::PatternEncoder, filter::threshold::ThresholdFilter, Config};

use crate::common::{ArraySize, StrPtr};

const MAX_EXCEPTIONS_COUNT: usize = 10;

struct LastExceptionMesssages {
    msgs: [StrPtr; MAX_EXCEPTIONS_COUNT],
    count: usize
}

impl LastExceptionMesssages {
    const fn new() -> Self {
        LastExceptionMesssages {
            msgs: [std::ptr::null_mut(); MAX_EXCEPTIONS_COUNT],
            count: 0,
        }
    }

    fn append(&mut self, msg: String) {
        if self.count < MAX_EXCEPTIONS_COUNT {
            match CString::new(msg) {
                Ok(msg) => {
                    self.msgs[self.count] = CString::into_raw(msg);
                    self.count += 1;
                },
                Err(err) => {
                    error!("Can't append exception: {err}");
                },
            }
        } else {
            error!("Can't append more exceptions we already have {MAX_EXCEPTIONS_COUNT}");
        }
    }

    fn clear(&mut self) {
        for i in 0..self.count {
            let msg = unsafe {
                CString::from_raw(self.msgs[i])
            };
            std::mem::drop(msg);
            self.msgs[i] = std::ptr::null_mut();
        }
        self.count = 0;
    }

    fn exceptions_array(&self) -> ExceptionsArray {
        ExceptionsArray {
            items: self.msgs.as_ptr(),
            count: self.count as ArraySize,
        }
    }
}

thread_local! {
    static LAST_EXCEPTION_MSGS: RefCell<LastExceptionMesssages> = const { RefCell::new(LastExceptionMesssages::new()) };
}

fn append_exception_msg(msg: String) {
    LAST_EXCEPTION_MSGS.with_borrow_mut(|last_exception_msgs| {
        last_exception_msgs.append(msg);
    });
}

fn clear_exception_msgs() {
    LAST_EXCEPTION_MSGS.with_borrow_mut(|last_exception_msgs| {
        last_exception_msgs.clear();
    });
}

#[repr(C)]
pub struct ExceptionsArray {
    items: *const StrPtr,
    count: ArraySize
}

#[no_mangle]
pub extern "C" fn logger_check_exceptions() -> ExceptionsArray {
    let result = std::panic::catch_unwind(|| {
        LAST_EXCEPTION_MSGS.with_borrow(|last_exception_messages| {
            last_exception_messages.exceptions_array()
        })
    });
    match result {
        Ok(result) => result,
        Err(payload) => {
            let msg = panic_payload_msg(payload);
            error!("logger_check_exceptions panic with: {msg}");
            ExceptionsArray {
                items: std::ptr::null(),
                count: 0,
            }
        },
    }
}

#[no_mangle]
pub extern "C" fn logger_clear_exceptions() {
    clear_exception_msgs();
}

#[allow(dead_code)]
#[repr(C)]
enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Off => logger::LevelFilter::Off,
            LogLevel::Error => log::LevelFilter::Error,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

#[repr(C)]
pub struct LoggerConfiguration {
    file_path: StrPtr,
    console_level: LogLevel,
    file_level: LogLevel
}

impl LoggerConfiguration {
    fn file_path(&self) -> anyhow::Result<&str> {
        let c_str = unsafe { CStr::from_ptr(self.file_path) };
        c_str.to_str().with_context(|| {
            format!("Invalid unicode in {c_str:?}")
        })
    }

    fn console_log_level(&self) -> log::LevelFilter {
        self.console_level.level_filter()
    }

    fn file_log_level(&self) -> log::LevelFilter {
        self.file_level.level_filter()
    }

    fn file_appender(&self) -> anyhow::Result<FileAppender> {
        let file_path = self.file_path()?;
        return  FileAppender::builder()
            // Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html
            .encoder(Box::new(PatternEncoder::new("[{d(%Y%m%d %H:%M:%S%.6f)} {h({l:5})} {M}:{L}] {m}{n}")))
            .build(file_path).context("Failed to create file appender");
    }

    fn console_appender(&self) -> ConsoleAppender {
        return ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new("[{d(%Y%m%d %H:%M:%S%.3f)} {h({l:5})} {M}:{L}] {m}{n}")))
            .target(Target::Stderr).build()
    }
}

#[no_mangle]
pub extern "C" fn logger_init(logger_configuration: &LoggerConfiguration) {
    let result = std::panic::catch_unwind(|| {
        let console_level = logger_configuration.console_log_level();
        let file_level = logger_configuration.file_log_level();

        let mut appenders = vec![];

        let console_appender = logger_configuration.console_appender();
        appenders.push(
            Appender::builder()
            .filter(Box::new(ThresholdFilter::new(console_level)))
            .build("stderr", Box::new(console_appender))
        );

        match logger_configuration.file_appender() {
            Ok(file_appender) => {
                appenders.push(
                    Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(file_level)))
                    .build("logfile", Box::new(file_appender)));
            },
            Err(err) => {
                append_exception_msg(format!("File appender creatrion failed: {err}"));
            },
        }

        let appender_names: Vec<_> = appenders.iter().map(|a| a.name().to_string()).collect();
        let config = Config::builder()
            .appenders(appenders)
            .build(
                Root::builder()
                    .appenders(appender_names)
                    .build(std::cmp::max(console_level, file_level)),
            );
        match config {
            Ok(config) => {
                match log4rs::init_config(config) {
                    Ok(_handle) => {
                        // todo store handler and allow to change logger severity
                    },
                    Err(err) => {
                        append_exception_msg(format!("Failed to init logger: {err}"));
                    },
                }
            },
            Err(err) => {
                append_exception_msg(format!("Failed to create logger config: {err}"));
            },
        };
    });

    match result {
        Err(payload) => {
            let msg = panic_payload_msg(payload);
            append_exception_msg(format!("logger_init panic with payload: {msg}"));
        },
        _ => {}
    }
    init_panic_handler();
    info!("Logger initialized");
}

fn panic_payload_msg(payload: Box<dyn Any + Send + 'static>) -> String {
    return if let Some(msg) = payload.downcast_ref::<&str>() {
        msg.to_string()
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.to_string()
    } else {
        format!("{payload:?}")
    };
}


pub(crate) trait PanicDefault {
    fn default() -> Self;
}

impl PanicDefault for () {
    fn default() -> Self {
        return ();
    }
}

// This function ignores [`UnwindSafe`] which means that in case of panic
// some mutable data types invariants might be violated.
// E.g. thread withdraw an ammount form one account and panicked before entering it to another account.
pub(crate) fn ffi_boundary<R: PanicDefault, F: FnOnce() -> anyhow::Result<R>>(name: &str, f: F) -> R {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => {
            let err = err.context(format!("{name:?} returned error"));
            let message = format!("{err:#}");
            append_exception_msg(message);
            PanicDefault::default()
        }
        Err(payload) => {
            let payload_msg = panic_payload_msg(payload);
            let message = format!("{name:?} panic with payload: {payload_msg}");
            append_exception_msg(message);
            PanicDefault::default()
        },
    }
}

pub(crate) fn init_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");
        let backtrace = std::backtrace::Backtrace::force_capture();
        error!("thread = {thread_name}, {panic_info}, Unhandled panic\n{backtrace}");
        log::logger().flush();
    }));
}