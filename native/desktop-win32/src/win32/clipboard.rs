use std::fmt::{Display, Formatter};

use desktop_common::{
    ffi_utils::{AutoDropArray, RustAllocatedStrPtr},
    logger::PanicDefault,
};
use windows::Win32::Foundation::{
    CLIPBRD_E_BAD_DATA, CLIPBRD_E_CANT_CLOSE, CLIPBRD_E_CANT_EMPTY, CLIPBRD_E_CANT_OPEN, CLIPBRD_E_CANT_SET, DV_E_CLIPFORMAT,
    DV_E_FORMATETC, DV_E_TYMED,
};
use windows_core::Error as WinError;

use super::{com::ComInterfaceRawPtr, data_object_api::AutoDropByteArray};

pub(crate) type AutoDropStringArray = AutoDropArray<RustAllocatedStrPtr>;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardStatus {
    Ok,
    Busy,
    FormatUnavailable,
    DataTooLarge,
    InvalidData,
    NativeError,
}

impl PanicDefault for ClipboardStatus {
    fn default() -> Self {
        Self::NativeError
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ClipboardOperationResult {
    pub status: ClipboardStatus,
    pub code: i32,
    pub message: RustAllocatedStrPtr,
}

impl ClipboardOperationResult {
    #[must_use]
    pub const fn ok() -> Self {
        Self {
            status: ClipboardStatus::Ok,
            code: 0,
            message: RustAllocatedStrPtr::null(),
        }
    }

    #[must_use]
    pub const fn failed(status: ClipboardStatus, code: i32) -> Self {
        Self {
            status,
            code,
            message: RustAllocatedStrPtr::null(),
        }
    }

    #[must_use]
    pub fn failed_with_message(status: ClipboardStatus, code: i32, message: &impl ToString) -> Self {
        let message = message.to_string().replace('\0', "\\0");
        Self {
            status,
            code,
            message: RustAllocatedStrPtr::allocate(message).unwrap_or_else(|_| RustAllocatedStrPtr::null()),
        }
    }

    #[must_use]
    pub fn from_error(err: &anyhow::Error) -> Self {
        if let Some(failure) = find_error::<ClipboardFailure>(err) {
            return failure.to_operation_result();
        }

        if let Some(win_error) = find_error::<WinError>(err) {
            return Self::from_win_error(win_error, err);
        }

        Self::failed_with_message(ClipboardStatus::NativeError, 0, err)
    }

    #[must_use]
    pub fn from_win_error(err: &WinError, message: &impl ToString) -> Self {
        let code = err.code();
        let status = match code {
            CLIPBRD_E_CANT_OPEN | CLIPBRD_E_CANT_EMPTY | CLIPBRD_E_CANT_SET | CLIPBRD_E_CANT_CLOSE => ClipboardStatus::Busy,
            DV_E_FORMATETC | DV_E_TYMED | DV_E_CLIPFORMAT => ClipboardStatus::FormatUnavailable,
            CLIPBRD_E_BAD_DATA => ClipboardStatus::InvalidData,
            _ => ClipboardStatus::NativeError,
        };
        Self::failed_with_message(status, code.0, message)
    }
}

impl PanicDefault for ClipboardOperationResult {
    fn default() -> Self {
        Self::failed(ClipboardStatus::NativeError, 0)
    }
}

#[derive(Debug)]
pub struct ClipboardFailure {
    status: ClipboardStatus,
    code: i32,
    message: String,
}

impl ClipboardFailure {
    #[must_use]
    pub fn data_too_large(size: usize, max_size: usize) -> Self {
        Self {
            status: ClipboardStatus::DataTooLarge,
            code: 0,
            message: format!("clipboard data is too large: {size} bytes exceeds {max_size} bytes"),
        }
    }

    #[must_use]
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self {
            status: ClipboardStatus::InvalidData,
            code: 0,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn to_operation_result(&self) -> ClipboardOperationResult {
        ClipboardOperationResult::failed_with_message(self.status, self.code, &self.message)
    }

    #[must_use]
    pub const fn status(&self) -> ClipboardStatus {
        self.status
    }
}

impl Display for ClipboardFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ClipboardFailure {}

/// Generates the `from_result` constructor and [`PanicDefault`] impl shared by every
/// `Clipboard*Result` FFI struct: on `Ok` the value is wrapped with a successful
/// [`ClipboardOperationResult`]; on `Err` the error is traced, classified via
/// [`ClipboardOperationResult::from_error`], and the value falls back to its `Default`.
///
/// The `#[repr(C)]` struct definitions stay spelled out at each call site so cbindgen still sees
/// them; only the otherwise-identical impl bodies are generated here.
macro_rules! clipboard_value_result {
    ($name:ident, $value:ty) => {
        impl $name {
            #[must_use]
            pub fn from_result(result: anyhow::Result<$value>) -> Self {
                match result {
                    Ok(value) => Self {
                        result: ClipboardOperationResult::ok(),
                        value,
                    },
                    Err(err) => {
                        trace_clipboard_failure(&err);
                        Self {
                            result: ClipboardOperationResult::from_error(&err),
                            value: <$value as PanicDefault>::default(),
                        }
                    }
                }
            }
        }

        impl PanicDefault for $name {
            fn default() -> Self {
                Self {
                    result: ClipboardOperationResult::default(),
                    value: <$value as PanicDefault>::default(),
                }
            }
        }
    };
}

#[repr(C)]
pub struct ClipboardBoolResult {
    pub result: ClipboardOperationResult,
    pub value: bool,
}
clipboard_value_result!(ClipboardBoolResult, bool);

#[repr(C)]
pub struct ClipboardByteArrayResult {
    pub result: ClipboardOperationResult,
    pub value: AutoDropByteArray,
}
clipboard_value_result!(ClipboardByteArrayResult, AutoDropByteArray);

#[repr(C)]
pub struct ClipboardStringResult {
    pub result: ClipboardOperationResult,
    pub value: RustAllocatedStrPtr,
}
clipboard_value_result!(ClipboardStringResult, RustAllocatedStrPtr);

#[repr(C)]
pub struct ClipboardStringArrayResult {
    pub result: ClipboardOperationResult,
    pub value: AutoDropStringArray,
}
clipboard_value_result!(ClipboardStringArrayResult, AutoDropStringArray);

#[repr(C)]
pub struct ClipboardUInt32ArrayResult {
    pub result: ClipboardOperationResult,
    pub value: AutoDropArray<u32>,
}
clipboard_value_result!(ClipboardUInt32ArrayResult, AutoDropArray<u32>);

#[repr(C)]
pub struct ClipboardDataObjectResult {
    pub result: ClipboardOperationResult,
    pub value: ComInterfaceRawPtr,
}
clipboard_value_result!(ClipboardDataObjectResult, ComInterfaceRawPtr);

pub(crate) fn operation_result(result: anyhow::Result<()>) -> ClipboardOperationResult {
    match result {
        Ok(()) => ClipboardOperationResult::ok(),
        Err(err) => {
            trace_clipboard_failure(&err);
            ClipboardOperationResult::from_error(&err)
        }
    }
}

fn trace_clipboard_failure(err: &anyhow::Error) {
    log::trace!("clipboard operation failed: {err:?}");
}

fn find_error<T: std::error::Error + 'static>(err: &anyhow::Error) -> Option<&T> {
    err.chain().find_map(|cause| cause.downcast_ref::<T>())
}
