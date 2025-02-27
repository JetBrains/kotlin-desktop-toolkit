use std::{any::Any, cell::RefCell, panic::AssertUnwindSafe};

use anyhow::Context;
use log::error;
use log4rs::{
    Config,
    append::{
        console::{ConsoleAppender, Target},
        rolling_file::{
            RollingFileAppender,
            policy::compound::{CompoundPolicy, roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger},
        },
    },
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};

use crate::logger_api::{LogLevel, LoggerConfiguration};
use crate::{
    common::{ArraySize, RustAllocatedStrPtr},
    logger_api::ExceptionsArray,
};

const MAX_EXCEPTIONS_COUNT: usize = 10;

struct LastExceptionMessages {
    messages: [RustAllocatedStrPtr; MAX_EXCEPTIONS_COUNT],
    count: usize,
}

impl LastExceptionMessages {
    const fn new() -> Self {
        Self {
            messages: [RustAllocatedStrPtr::null(); MAX_EXCEPTIONS_COUNT],
            count: 0,
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn append(&mut self, msg: String) {
        if self.count < MAX_EXCEPTIONS_COUNT {
            match RustAllocatedStrPtr::allocate(msg.as_bytes()) {
                Ok(msg) => {
                    self.messages[self.count] = msg;
                    self.count += 1;
                }
                Err(err) => {
                    error!("Can't append exception: {err}");
                }
            }
        } else {
            error!("Can't append more exceptions we already have {MAX_EXCEPTIONS_COUNT}");
        }
    }

    fn clear(&mut self) {
        for i in 0..self.count {
            self.messages[i].deallocate();
            self.messages[i] = RustAllocatedStrPtr::null();
        }
        self.count = 0;
    }

    pub const fn exceptions_array(&self) -> ExceptionsArray {
        ExceptionsArray {
            items: self.messages.as_ptr(),
            count: self.count as ArraySize,
        }
    }
}

thread_local! {
    pub static LAST_EXCEPTION_MSGS: RefCell<LastExceptionMessages> = const { RefCell::new(LastExceptionMessages::new()) };
}

pub fn append_exception_msg(msg: String) {
    LAST_EXCEPTION_MSGS.with_borrow_mut(|last_exception_msgs| {
        last_exception_msgs.append(msg);
    });
}

pub fn clear_exception_msgs() {
    LAST_EXCEPTION_MSGS.with_borrow_mut(|last_exception_msgs| {
        last_exception_msgs.clear();
    });
}

pub fn exceptions_array() -> ExceptionsArray {
    LAST_EXCEPTION_MSGS.with_borrow(LastExceptionMessages::exceptions_array)
}

impl LogLevel {
    const fn level_filter(&self) -> log::LevelFilter {
        match self {
            Self::Off => log::LevelFilter::Off,
            Self::Error => log::LevelFilter::Error,
            Self::Warn => log::LevelFilter::Warn,
            Self::Info => log::LevelFilter::Info,
            Self::Debug => log::LevelFilter::Debug,
            Self::Trace => log::LevelFilter::Trace,
        }
    }
}

impl LoggerConfiguration<'_> {
    fn file_path(&self) -> anyhow::Result<&str> {
        self.file_path.as_str()
    }

    const fn console_log_level(&self) -> log::LevelFilter {
        self.console_level.level_filter()
    }

    const fn file_log_level(&self) -> log::LevelFilter {
        self.file_level.level_filter()
    }

    fn file_appender(&self) -> anyhow::Result<RollingFileAppender> {
        const TRIGGER_FILE_SIZE: u64 = 2 * 1024 * 1024; // 2Mb
        const LOG_FILE_COUNT: u32 = 3;

        let file_path = std::path::Path::new(self.file_path()?);

        let file_name = file_path
            .file_stem()
            .context("File expected")?
            .to_str()
            .context("Can't convert OS string")?;
        let archive_pattern = file_path.with_file_name(format!("{file_name}{{}}.log"));

        let trigger = SizeTrigger::new(TRIGGER_FILE_SIZE);
        let roller = FixedWindowRoller::builder()
            .build(
                archive_pattern.to_str().context("file_path contains invalid unicode")?,
                LOG_FILE_COUNT,
            )
            .unwrap();
        let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

        RollingFileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "[{d(%Y%m%d %H:%M:%S%.6f)} {h({l:5})} {M}:{L}] {m}{n}",
            )))
            .build(file_path, Box::new(policy))
            .context("Failed to create file appender")
    }

    fn console_appender() -> ConsoleAppender {
        ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "[{d(%Y%m%d %H:%M:%S%.3f)} {h({l:5})} {M}:{L}] {m}{n}",
            )))
            .target(Target::Stderr)
            .build()
    }

    pub(crate) fn init_logger(&self) {
        unsafe {
            // enable backtraces for anyhow errors
            std::env::set_var("RUST_LIB_BACKTRACE", "1");
        }
        let console_level = self.console_log_level();
        let file_level = self.file_log_level();

        let mut appenders = vec![];

        let console_appender = Self::console_appender();
        appenders.push(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(console_level)))
                .build("stderr", Box::new(console_appender)),
        );

        match self.file_appender() {
            Ok(file_appender) => {
                appenders.push(
                    Appender::builder()
                        .filter(Box::new(ThresholdFilter::new(file_level)))
                        .build("logfile", Box::new(file_appender)),
                );
            }
            Err(err) => {
                append_exception_msg(format!("File appender creatrion failed: {err}"));
            }
        }

        let appender_names: Vec<_> = appenders.iter().map(|a| a.name().to_string()).collect();
        let config = Config::builder().appenders(appenders).build(
            Root::builder()
                .appenders(appender_names)
                .build(std::cmp::max(console_level, file_level)),
        );
        match config {
            Ok(config) => {
                match log4rs::init_config(config) {
                    Ok(_handle) => {
                        // todo store handler and allow to change logger severity
                    }
                    Err(err) => {
                        append_exception_msg(format!("Failed to init logger: {err}"));
                    }
                }
            }
            Err(err) => {
                append_exception_msg(format!("Failed to create logger config: {err}"));
            }
        };
    }
}

pub fn panic_payload_msg(payload: &Box<dyn Any + Send + 'static>) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        (*msg).to_string()
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.clone()
    } else {
        format!("{payload:?}")
    }
}

// This function intended to stop and log panic when out code is called from Objective C
// otherwise it will terminate the application
pub fn catch_panic<R, F: FnOnce() -> anyhow::Result<R>>(f: F) -> Option<R> {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(result)) => Some(result),
        Ok(Err(err)) => {
            error!("{err:?}");
            None
        }
        Err(_payload) => {
            // do nothing, panic will be logged by handler
            None
        }
    }
}

pub trait PanicDefault {
    fn default() -> Self;
}

impl PanicDefault for () {
    fn default() -> Self {}
}

impl<T> PanicDefault for Option<T> {
    fn default() -> Self {
        None
    }
}

// We wrap body of API functions that we expose with this function, e.g. see `application_init`
// It prevents Rust panic from crossing the border
// This function ignores [`UnwindSafe`] which means that in case of panic
// some mutable data types invariants might be violated.
// E.g. thread withdraw an amount form one account and panicked before entering it to another account.
pub fn ffi_boundary<R: PanicDefault, F: FnOnce() -> anyhow::Result<R>>(name: &str, f: F) -> R {
    match std::panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => {
            let err = err.context(format!("{name:?} returned error"));
            error!("{err:?}");
            append_exception_msg(format!("{err:#}"));
            PanicDefault::default()
        }
        Err(payload) => {
            let payload_msg = panic_payload_msg(&payload);
            let message = format!("{name:?} panic with payload: {payload_msg}");
            append_exception_msg(message); // message will be also logged by panic handler
            PanicDefault::default()
        }
    }
}

pub fn init_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("<unnamed>");
        let backtrace = std::backtrace::Backtrace::force_capture();
        error!("thread = {thread_name}, {panic_info}, Unhandled panic\n{backtrace}");
        log::logger().flush();
    }));
}
