use std::fmt::{Display, Formatter};

use desktop_common::{
    ffi_utils::{AutoDropArray, RustAllocatedStrPtr},
    logger::PanicDefault,
};
use windows_core::Error as WinError;

use super::{com::ComInterfaceRawPtr, data_object_api::AutoDropByteArray};

pub(crate) type AutoDropStringArray = AutoDropArray<RustAllocatedStrPtr>;

/// cbindgen:ignore
const HRESULT_CLIPBRD_E_CANT_OPEN: i32 = 0x8004_01D0_u32 as i32;
/// cbindgen:ignore
const HRESULT_CLIPBRD_E_CANT_EMPTY: i32 = 0x8004_01D1_u32 as i32;
/// cbindgen:ignore
const HRESULT_CLIPBRD_E_CANT_SET: i32 = 0x8004_01D2_u32 as i32;
/// cbindgen:ignore
const HRESULT_CLIPBRD_E_BAD_DATA: i32 = 0x8004_01D3_u32 as i32;
/// cbindgen:ignore
const HRESULT_CLIPBRD_E_CANT_CLOSE: i32 = 0x8004_01D4_u32 as i32;
/// cbindgen:ignore
const HRESULT_DV_E_FORMATETC: i32 = 0x8004_0064_u32 as i32;
/// cbindgen:ignore
const HRESULT_DV_E_TYMED: i32 = 0x8004_0069_u32 as i32;
/// cbindgen:ignore
const HRESULT_DV_E_CLIPFORMAT: i32 = 0x8004_006A_u32 as i32;

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
        let code = err.code().0;
        let status = match code {
            HRESULT_CLIPBRD_E_CANT_OPEN | HRESULT_CLIPBRD_E_CANT_EMPTY | HRESULT_CLIPBRD_E_CANT_SET | HRESULT_CLIPBRD_E_CANT_CLOSE => {
                ClipboardStatus::Busy
            }
            HRESULT_DV_E_FORMATETC | HRESULT_DV_E_TYMED | HRESULT_DV_E_CLIPFORMAT => ClipboardStatus::FormatUnavailable,
            HRESULT_CLIPBRD_E_BAD_DATA => ClipboardStatus::InvalidData,
            _ => ClipboardStatus::NativeError,
        };
        Self::failed_with_message(status, code, message)
    }

    #[must_use]
    pub const fn is_ok(&self) -> bool {
        matches!(self.status, ClipboardStatus::Ok)
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

#[repr(C)]
pub struct ClipboardBoolResult {
    pub result: ClipboardOperationResult,
    pub value: bool,
}

impl ClipboardBoolResult {
    #[must_use]
    pub fn from_result(result: anyhow::Result<bool>) -> Self {
        match result {
            Ok(value) => Self {
                result: ClipboardOperationResult::ok(),
                value,
            },
            Err(err) => {
                trace_clipboard_failure(&err);
                Self {
                    result: ClipboardOperationResult::from_error(&err),
                    value: <bool as Default>::default(),
                }
            }
        }
    }
}

impl PanicDefault for ClipboardBoolResult {
    fn default() -> Self {
        Self {
            result: ClipboardOperationResult::default(),
            value: <bool as Default>::default(),
        }
    }
}

#[repr(C)]
pub struct ClipboardByteArrayResult {
    pub result: ClipboardOperationResult,
    pub value: AutoDropByteArray,
}

impl ClipboardByteArrayResult {
    #[must_use]
    pub fn from_result(result: anyhow::Result<AutoDropByteArray>) -> Self {
        match result {
            Ok(value) => Self {
                result: ClipboardOperationResult::ok(),
                value,
            },
            Err(err) => {
                trace_clipboard_failure(&err);
                Self {
                    result: ClipboardOperationResult::from_error(&err),
                    value: AutoDropByteArray::default(),
                }
            }
        }
    }
}

impl PanicDefault for ClipboardByteArrayResult {
    fn default() -> Self {
        Self {
            result: ClipboardOperationResult::default(),
            value: AutoDropByteArray::default(),
        }
    }
}

#[repr(C)]
pub struct ClipboardStringResult {
    pub result: ClipboardOperationResult,
    pub value: RustAllocatedStrPtr,
}

impl ClipboardStringResult {
    #[must_use]
    pub fn from_result(result: anyhow::Result<RustAllocatedStrPtr>) -> Self {
        match result {
            Ok(value) => Self {
                result: ClipboardOperationResult::ok(),
                value,
            },
            Err(err) => {
                trace_clipboard_failure(&err);
                Self {
                    result: ClipboardOperationResult::from_error(&err),
                    value: RustAllocatedStrPtr::default(),
                }
            }
        }
    }
}

impl PanicDefault for ClipboardStringResult {
    fn default() -> Self {
        Self {
            result: ClipboardOperationResult::default(),
            value: RustAllocatedStrPtr::default(),
        }
    }
}

#[repr(C)]
pub struct ClipboardStringArrayResult {
    pub result: ClipboardOperationResult,
    pub value: AutoDropStringArray,
}

impl ClipboardStringArrayResult {
    #[must_use]
    pub fn from_result(result: anyhow::Result<AutoDropStringArray>) -> Self {
        match result {
            Ok(value) => Self {
                result: ClipboardOperationResult::ok(),
                value,
            },
            Err(err) => {
                trace_clipboard_failure(&err);
                Self {
                    result: ClipboardOperationResult::from_error(&err),
                    value: AutoDropStringArray::default(),
                }
            }
        }
    }
}

impl PanicDefault for ClipboardStringArrayResult {
    fn default() -> Self {
        Self {
            result: ClipboardOperationResult::default(),
            value: AutoDropStringArray::default(),
        }
    }
}

#[repr(C)]
pub struct ClipboardUInt32ArrayResult {
    pub result: ClipboardOperationResult,
    pub value: AutoDropArray<u32>,
}

impl ClipboardUInt32ArrayResult {
    #[must_use]
    pub fn from_result(result: anyhow::Result<AutoDropArray<u32>>) -> Self {
        match result {
            Ok(value) => Self {
                result: ClipboardOperationResult::ok(),
                value,
            },
            Err(err) => {
                trace_clipboard_failure(&err);
                Self {
                    result: ClipboardOperationResult::from_error(&err),
                    value: AutoDropArray::default(),
                }
            }
        }
    }
}

impl PanicDefault for ClipboardUInt32ArrayResult {
    fn default() -> Self {
        Self {
            result: ClipboardOperationResult::default(),
            value: AutoDropArray::default(),
        }
    }
}

#[repr(C)]
pub struct ClipboardDataObjectResult {
    pub result: ClipboardOperationResult,
    pub value: ComInterfaceRawPtr,
}

impl ClipboardDataObjectResult {
    #[must_use]
    pub fn from_result(result: anyhow::Result<ComInterfaceRawPtr>) -> Self {
        match result {
            Ok(value) => Self {
                result: ClipboardOperationResult::ok(),
                value,
            },
            Err(err) => {
                trace_clipboard_failure(&err);
                Self {
                    result: ClipboardOperationResult::from_error(&err),
                    value: ComInterfaceRawPtr::default(),
                }
            }
        }
    }
}

impl PanicDefault for ClipboardDataObjectResult {
    fn default() -> Self {
        Self {
            result: ClipboardOperationResult::default(),
            value: ComInterfaceRawPtr::default(),
        }
    }
}

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
