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
pub enum TransferStatus {
    Ok,
    Busy,
    FormatUnavailable,
    DataTooLarge,
    InvalidData,
    NativeError,
}

impl PanicDefault for TransferStatus {
    fn default() -> Self {
        Self::NativeError
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TransferOperationResult {
    pub status: TransferStatus,
    pub code: i32,
    pub message: RustAllocatedStrPtr,
}

impl TransferOperationResult {
    #[must_use]
    pub const fn ok() -> Self {
        Self {
            status: TransferStatus::Ok,
            code: 0,
            message: RustAllocatedStrPtr::null(),
        }
    }

    #[must_use]
    pub const fn failed(status: TransferStatus, code: i32) -> Self {
        Self {
            status,
            code,
            message: RustAllocatedStrPtr::null(),
        }
    }

    #[must_use]
    pub fn failed_with_message(status: TransferStatus, code: i32, message: &impl ToString) -> Self {
        let message = message.to_string().replace('\0', "\\0");
        Self {
            status,
            code,
            message: RustAllocatedStrPtr::allocate(message).unwrap_or_else(|_| RustAllocatedStrPtr::null()),
        }
    }

    #[must_use]
    pub fn from_error(err: &anyhow::Error) -> Self {
        if let Some(failure) = find_error::<TransferFailure>(err) {
            return failure.to_operation_result();
        }

        if let Some(win_error) = find_error::<WinError>(err) {
            return Self::from_win_error(win_error, err);
        }

        Self::failed_with_message(TransferStatus::NativeError, 0, err)
    }

    #[must_use]
    pub fn from_win_error(err: &WinError, message: &impl ToString) -> Self {
        let code = err.code();
        let status = match code {
            CLIPBRD_E_CANT_OPEN | CLIPBRD_E_CANT_EMPTY | CLIPBRD_E_CANT_SET | CLIPBRD_E_CANT_CLOSE => TransferStatus::Busy,
            DV_E_FORMATETC | DV_E_TYMED | DV_E_CLIPFORMAT => TransferStatus::FormatUnavailable,
            CLIPBRD_E_BAD_DATA => TransferStatus::InvalidData,
            _ => TransferStatus::NativeError,
        };
        Self::failed_with_message(status, code.0, message)
    }
}

impl PanicDefault for TransferOperationResult {
    fn default() -> Self {
        Self::failed(TransferStatus::NativeError, 0)
    }
}

#[derive(Debug)]
pub struct TransferFailure {
    status: TransferStatus,
    code: i32,
    message: String,
}

impl TransferFailure {
    #[must_use]
    pub fn data_too_large(size: usize, max_size: usize) -> Self {
        Self {
            status: TransferStatus::DataTooLarge,
            code: 0,
            message: format!("clipboard data is too large: {size} bytes exceeds {max_size} bytes"),
        }
    }

    #[must_use]
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self {
            status: TransferStatus::InvalidData,
            code: 0,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn to_operation_result(&self) -> TransferOperationResult {
        TransferOperationResult::failed_with_message(self.status, self.code, &self.message)
    }

    #[must_use]
    pub const fn status(&self) -> TransferStatus {
        self.status
    }
}

impl Display for TransferFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for TransferFailure {}

/// Generates the `from_result` constructor and [`PanicDefault`] impl shared by every
/// `Transfer*Result` FFI struct: on `Ok` the value is wrapped with a successful
/// [`TransferOperationResult`]; on `Err` the error is traced, classified via
/// [`TransferOperationResult::from_error`], and the value falls back to its `Default`.
///
/// The `#[repr(C)]` struct definitions stay spelled out at each call site so cbindgen still sees
/// them; only the otherwise-identical impl bodies are generated here.
macro_rules! transfer_value_result {
    ($name:ident, $value:ty) => {
        impl $name {
            #[must_use]
            pub fn from_result(result: anyhow::Result<$value>) -> Self {
                match result {
                    Ok(value) => Self {
                        result: TransferOperationResult::ok(),
                        value,
                    },
                    Err(err) => {
                        trace_transfer_failure(&err);
                        Self {
                            result: TransferOperationResult::from_error(&err),
                            value: <$value as PanicDefault>::default(),
                        }
                    }
                }
            }
        }

        impl PanicDefault for $name {
            fn default() -> Self {
                Self {
                    result: TransferOperationResult::default(),
                    value: <$value as PanicDefault>::default(),
                }
            }
        }
    };
}

#[repr(C)]
pub struct TransferBoolResult {
    pub result: TransferOperationResult,
    pub value: bool,
}
transfer_value_result!(TransferBoolResult, bool);

#[repr(C)]
pub struct TransferByteArrayResult {
    pub result: TransferOperationResult,
    pub value: AutoDropByteArray,
}
transfer_value_result!(TransferByteArrayResult, AutoDropByteArray);

#[repr(C)]
pub struct TransferStringResult {
    pub result: TransferOperationResult,
    pub value: RustAllocatedStrPtr,
}
transfer_value_result!(TransferStringResult, RustAllocatedStrPtr);

#[repr(C)]
pub struct TransferStringArrayResult {
    pub result: TransferOperationResult,
    pub value: AutoDropStringArray,
}
transfer_value_result!(TransferStringArrayResult, AutoDropStringArray);

#[repr(C)]
pub struct TransferUInt32ArrayResult {
    pub result: TransferOperationResult,
    pub value: AutoDropArray<u32>,
}
transfer_value_result!(TransferUInt32ArrayResult, AutoDropArray<u32>);

#[repr(C)]
pub struct TransferDataObjectResult {
    pub result: TransferOperationResult,
    pub value: ComInterfaceRawPtr,
}
transfer_value_result!(TransferDataObjectResult, ComInterfaceRawPtr);

pub(crate) fn operation_result(result: anyhow::Result<()>) -> TransferOperationResult {
    match result {
        Ok(()) => TransferOperationResult::ok(),
        Err(err) => {
            trace_transfer_failure(&err);
            TransferOperationResult::from_error(&err)
        }
    }
}

fn trace_transfer_failure(err: &anyhow::Error) {
    log::trace!("clipboard operation failed: {err:?}");
}

fn find_error<T: std::error::Error + 'static>(err: &anyhow::Error) -> Option<&T> {
    err.chain().find_map(|cause| cause.downcast_ref::<T>())
}
