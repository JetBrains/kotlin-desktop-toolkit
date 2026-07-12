use std::fmt::{Display, Formatter};
use std::sync::LazyLock;

use desktop_common::{
    ffi_utils::{AutoDropArray, RustAllocatedStrPtr},
    logger::{PanicDefault, ffi_boundary},
};
use windows::Win32::{
    Foundation::{
        CLIPBRD_E_BAD_DATA, CLIPBRD_E_CANT_CLOSE, CLIPBRD_E_CANT_EMPTY, CLIPBRD_E_CANT_OPEN, CLIPBRD_E_CANT_SET, DV_E_CLIPFORMAT,
        DV_E_FORMATETC, DV_E_TYMED,
    },
    System::{
        DataExchange::RegisterClipboardFormatW,
        Ole::{CF_HDROP, CF_UNICODETEXT},
    },
};
use windows_core::{Error as WinError, HSTRING, h};

use super::{com::ComInterfaceRawPtr, data_object_api::AutoDropByteArray};

/// cbindgen:ignore
static HTML_FORMAT: LazyLock<u32> = LazyLock::new(|| register_format(h!("HTML Format")));

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    Text,
    FileList,
    HtmlFragment,
    Other(u32),
}

impl DataFormat {
    #[inline]
    #[must_use]
    pub fn id(self) -> u32 {
        match self {
            Self::Text => u32::from(CF_UNICODETEXT.0),
            Self::FileList => u32::from(CF_HDROP.0),
            Self::HtmlFragment => *HTML_FORMAT,
            Self::Other(id) => id,
        }
    }
}

#[must_use]
pub fn register_format(format_name: &HSTRING) -> u32 {
    unsafe { RegisterClipboardFormatW(format_name) }
}

pub(crate) type AutoDropStringArray = AutoDropArray<RustAllocatedStrPtr>;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTransferStatus {
    Ok,
    Busy,
    FormatUnavailable,
    DataTooLarge,
    InvalidData,
    NativeError,
}

impl PanicDefault for DataTransferStatus {
    fn default() -> Self {
        Self::NativeError
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DataTransferOperationResult {
    pub status: DataTransferStatus,
    pub code: i32,
    pub message: RustAllocatedStrPtr,
}

impl DataTransferOperationResult {
    #[must_use]
    pub const fn ok() -> Self {
        Self {
            status: DataTransferStatus::Ok,
            code: 0,
            message: RustAllocatedStrPtr::null(),
        }
    }

    #[must_use]
    pub const fn failed(status: DataTransferStatus, code: i32) -> Self {
        Self {
            status,
            code,
            message: RustAllocatedStrPtr::null(),
        }
    }

    #[must_use]
    pub fn failed_with_message(status: DataTransferStatus, code: i32, message: &impl ToString) -> Self {
        let message = message.to_string().replace('\0', "\\0");
        Self {
            status,
            code,
            message: RustAllocatedStrPtr::allocate(message).unwrap_or_else(|_| RustAllocatedStrPtr::null()),
        }
    }

    #[must_use]
    pub fn from_error(err: &anyhow::Error) -> Self {
        if let Some(failure) = find_error::<DataTransferFailure>(err) {
            return failure.to_operation_result();
        }

        if let Some(win_error) = find_error::<WinError>(err) {
            return Self::from_win_error(win_error, err);
        }

        Self::failed_with_message(DataTransferStatus::NativeError, 0, err)
    }

    #[must_use]
    pub fn from_win_error(err: &WinError, message: &impl ToString) -> Self {
        let code = err.code();
        let status = match code {
            CLIPBRD_E_CANT_OPEN | CLIPBRD_E_CANT_EMPTY | CLIPBRD_E_CANT_SET | CLIPBRD_E_CANT_CLOSE => DataTransferStatus::Busy,
            DV_E_FORMATETC | DV_E_TYMED | DV_E_CLIPFORMAT => DataTransferStatus::FormatUnavailable,
            CLIPBRD_E_BAD_DATA => DataTransferStatus::InvalidData,
            _ => DataTransferStatus::NativeError,
        };
        Self::failed_with_message(status, code.0, message)
    }
}

impl PanicDefault for DataTransferOperationResult {
    fn default() -> Self {
        Self::failed(DataTransferStatus::NativeError, 0)
    }
}

#[derive(Debug)]
pub struct DataTransferFailure {
    status: DataTransferStatus,
    code: i32,
    message: String,
}

impl DataTransferFailure {
    #[must_use]
    pub fn data_too_large(size: usize, max_size: usize) -> Self {
        Self {
            status: DataTransferStatus::DataTooLarge,
            code: 0,
            message: format!("clipboard data is too large: {size} bytes exceeds {max_size} bytes"),
        }
    }

    #[must_use]
    pub fn format_unavailable(format_id: u32) -> Self {
        Self {
            status: DataTransferStatus::FormatUnavailable,
            code: 0,
            message: format!("requested format is unavailable: {format_id}"),
        }
    }

    #[must_use]
    pub fn invalid_data(message: impl Into<String>) -> Self {
        Self::invalid_data_with_code(0, message)
    }

    /// Like [`Self::invalid_data`] but carrying a Windows error code (an `HRESULT`, e.g. from
    /// [`WinError::from_thread`]). Use when a failing Win32 call may have set a meaningful
    /// last-error; pass `0` when there is no underlying OS error.
    #[must_use]
    pub fn invalid_data_with_code(code: i32, message: impl Into<String>) -> Self {
        Self {
            status: DataTransferStatus::InvalidData,
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn to_operation_result(&self) -> DataTransferOperationResult {
        DataTransferOperationResult::failed_with_message(self.status, self.code, &self.message)
    }

    #[must_use]
    pub const fn status(&self) -> DataTransferStatus {
        self.status
    }
}

impl Display for DataTransferFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DataTransferFailure {}

/// Builds a `#[repr(C)]` data-transfer result struct from a fallible operation. Implemented by
/// every `DataTransfer*Result` value struct (via [`data_transfer_value_result!`]) and by
/// [`DataTransferOperationResult`] for `()`-valued operations, so [`data_transfer_boundary`] can
/// wrap any data-transfer FFI entry point generically.
pub(crate) trait DataTransferResult: PanicDefault {
    type Value;
    fn from_result(result: anyhow::Result<Self::Value>) -> Self;
}

/// Generates the [`DataTransferResult`] and [`PanicDefault`] impls shared by every
/// `DataTransfer*Result` FFI struct: on `Ok` the value is wrapped with a successful
/// [`DataTransferOperationResult`]; on `Err` the error is traced, classified via
/// [`DataTransferOperationResult::from_error`], and the value falls back to its `Default`.
///
/// The `#[repr(C)]` struct definitions stay spelled out at each call site so cbindgen still sees
/// them; only the otherwise-identical impl bodies are generated here.
macro_rules! data_transfer_value_result {
    ($name:ident, $value:ty) => {
        impl DataTransferResult for $name {
            type Value = $value;

            fn from_result(result: anyhow::Result<$value>) -> Self {
                match result {
                    Ok(value) => Self {
                        result: DataTransferOperationResult::ok(),
                        value,
                    },
                    Err(err) => {
                        trace_data_transfer_failure(&err);
                        Self {
                            result: DataTransferOperationResult::from_error(&err),
                            value: <$value as PanicDefault>::default(),
                        }
                    }
                }
            }
        }

        impl PanicDefault for $name {
            fn default() -> Self {
                Self {
                    result: DataTransferOperationResult::default(),
                    value: <$value as PanicDefault>::default(),
                }
            }
        }
    };
}

#[repr(C)]
pub struct DataTransferBoolResult {
    pub result: DataTransferOperationResult,
    pub value: bool,
}
data_transfer_value_result!(DataTransferBoolResult, bool);

#[repr(C)]
pub struct DataTransferByteArrayResult {
    pub result: DataTransferOperationResult,
    pub value: AutoDropByteArray,
}
data_transfer_value_result!(DataTransferByteArrayResult, AutoDropByteArray);

#[repr(C)]
pub struct DataTransferStringResult {
    pub result: DataTransferOperationResult,
    pub value: RustAllocatedStrPtr,
}
data_transfer_value_result!(DataTransferStringResult, RustAllocatedStrPtr);

#[repr(C)]
pub struct DataTransferStringArrayResult {
    pub result: DataTransferOperationResult,
    pub value: AutoDropStringArray,
}
data_transfer_value_result!(DataTransferStringArrayResult, AutoDropStringArray);

#[repr(C)]
pub struct DataTransferUInt32ArrayResult {
    pub result: DataTransferOperationResult,
    pub value: AutoDropArray<u32>,
}
data_transfer_value_result!(DataTransferUInt32ArrayResult, AutoDropArray<u32>);

#[repr(C)]
pub struct DataTransferDataObjectResult {
    pub result: DataTransferOperationResult,
    pub value: ComInterfaceRawPtr,
}
data_transfer_value_result!(DataTransferDataObjectResult, ComInterfaceRawPtr);

impl DataTransferResult for DataTransferOperationResult {
    type Value = ();

    fn from_result(result: anyhow::Result<()>) -> Self {
        match result {
            Ok(()) => Self::ok(),
            Err(err) => {
                trace_data_transfer_failure(&err);
                Self::from_error(&err)
            }
        }
    }
}

/// Wraps a data-transfer FFI entry point: runs `f` inside [`ffi_boundary`] (the panic guard) and
/// converts its `anyhow::Result` into the `#[repr(C)]` result struct `R` via [`DataTransferResult`],
/// so a failure is reported through the struct's status/code/message rather than as a panic default.
pub(crate) fn data_transfer_boundary<R: DataTransferResult>(name: &str, f: impl FnOnce() -> anyhow::Result<R::Value>) -> R {
    ffi_boundary(name, || Ok(R::from_result(f())))
}

fn trace_data_transfer_failure(err: &anyhow::Error) {
    log::trace!("clipboard operation failed: {err:?}");
}

fn find_error<T: std::error::Error + 'static>(err: &anyhow::Error) -> Option<&T> {
    err.chain().find_map(|cause| cause.downcast_ref::<T>())
}
