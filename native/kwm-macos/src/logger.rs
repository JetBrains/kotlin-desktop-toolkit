use core::panic;
use std::{any::Any, borrow::{Borrow, BorrowMut}, cell::{Cell, RefCell}, ffi::{CStr, CString}, fmt::format, panic::AssertUnwindSafe};

use anyhow::Context;
use log::{debug, error, info, trace, warn, Level, LevelFilter, Metadata, Record, SetLoggerError};
use log4rs::{append::{console::{ConsoleAppender, Target}, file::FileAppender}, config::{Appender, Root}, encode::pattern::PatternEncoder, filter::threshold::ThresholdFilter, Config};

use crate::common::{ArraySize, StrPtr};

#[repr(C)]
struct ExceptionsArray {
    items: *mut StrPtr,
    count: ArraySize
}

thread_local! {
    static LAST_EXCEPTION_MSGS: RefCell<ExceptionsArray> = const { RefCell::new(Vec::new()) };
}

fn append_exception_msg(msg: String) {
    CString::new(msg).into;

    LAST_EXCEPTION_MSGS.with_borrow_mut(|last_exception_msgs| {
        last_exception_msgs.push(msg);
    });
}

fn clear_exception_msgs() {
    LAST_EXCEPTION_MSGS.with_borrow_mut(|last_exception_msgs| {
        last_exception_msgs.clear();
    });
}

#[no_mangle]
pub extern "C" fn logger_check_exceptions() -> ExceptionsArray {
    let v = LAST_EXCEPTION_MSGS.borrow_mut();

}

#[no_mangle]
pub extern "C" fn logger_clear_exceptions() {
    clear_exception_msgs();
}

#[repr(C)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn level_filter(&self) -> log::LevelFilter {
        match self {
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
    level: LogLevel
}

impl LoggerConfiguration {
    fn file_path(&self) -> anyhow::Result<&str> {
        let c_str = unsafe { CStr::from_ptr(self.file_path) };
        c_str.to_str().with_context(|| {
            format!("Invalid unicode in {c_str:?}")
        })
    }

    fn log_level(&self) -> log::LevelFilter {
        self.level.level_filter()
    }

    fn file_appender(&self) -> anyhow::Result<FileAppender> {
        let file_path = self.file_path()?;
        return  FileAppender::builder()
            // Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html
            .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
            .build(file_path).context("Failed to create file appender");
    }
}

#[no_mangle]
pub extern "C" fn logger_init(logger_configuration: &LoggerConfiguration) {
    let result = std::panic::catch_unwind(|| {
        let level = logger_configuration.log_level();

        let mut appenders = vec![];

        let stderr = ConsoleAppender::builder().target(Target::Stderr).build();
        appenders.push(Appender::builder().build("stderr", Box::new(stderr)));

        match logger_configuration.file_appender() {
            Ok(file_appender) => {
                appenders.push(Appender::builder().build("logfile", Box::new(file_appender)));
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
                    .build(level),
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

// This function ignores [`UnwindSafe`] which means that in case of panic
// some mutable data types invariants might be violated.
// E.g. thread withdraw an ammount form one account and panicked before entering it to another account.
pub(crate) fn ffi_boundary<R: PanicDefault, F: FnOnce() -> anyhow::Result<R>>(name: &str, f: F) -> R {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => {
            let err = err.context(format!("{name:?} returned error"));
            let message = format!("{err:#}");
            error!("{message}");
            append_exception_msg(message);
            PanicDefault::default()
        }
        Err(payload) => {
            let payload_msg = panic_payload_msg(payload);
            let message = format!("{name:?} panic with payload: {payload_msg}");
            error!("{message}");
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
    }));
}