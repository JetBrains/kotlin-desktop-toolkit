use desktop_common::ffi_utils::BorrowedStrPtr;
use std::ffi::{CStr, CString};

use windows::Win32::{
    Foundation::{E_POINTER, E_STRING_NOT_NULL_TERMINATED, ERROR_NO_UNICODE_TRANSLATION},
    Globalization::{CP_UTF8, WC_ERR_INVALID_CHARS, WideCharToMultiByte},
};

#[allow(clippy::cast_sign_loss)]
pub(crate) fn copy_from_wide_string(s: &[u16]) -> windows::core::Result<CString> {
    let len = unsafe { WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, s, None, None, None) };
    if len == 0 {
        return Err(windows::core::Error::from_thread());
    }
    let mut buf = vec![0u8; len as usize + 1]; // ensure that we definitely have a terminating null character
    unsafe { WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, s, Some(&mut buf), None, None) };
    match CStr::from_bytes_until_nul(buf.as_slice()) {
        Ok(c_str) => Ok(c_str.to_owned()),
        Err(_) => Err(windows::core::Error::from(E_STRING_NOT_NULL_TERMINATED)),
    }
}

pub(crate) fn copy_from_utf8_string(s: &BorrowedStrPtr) -> windows::core::Result<windows::core::HSTRING> {
    s.as_optional_cstr()
        .ok_or_else(|| windows::core::Error::from(E_POINTER))
        .and_then(|c_str| c_str.to_str().map_err(|_| windows::core::Error::from(ERROR_NO_UNICODE_TRANSLATION)))
        .map(windows::core::HSTRING::from)
}
