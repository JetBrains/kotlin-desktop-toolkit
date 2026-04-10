use desktop_common::ffi_utils::BorrowedStrPtr;
use std::ffi::{CStr, CString};

use windows::Win32::{
    Foundation::{E_POINTER, E_STRING_NOT_NULL_TERMINATED, ERROR_NO_UNICODE_TRANSLATION},
    Globalization::{CP_UTF8, MB_ERR_INVALID_CHARS, MultiByteToWideChar, WC_ERR_INVALID_CHARS, WideCharToMultiByte},
};
use windows_core::{Error as WinError, HSTRING, Result as WinResult};

pub(crate) fn copy_from_wide_string(s: &[u16]) -> WinResult<CString> {
    let len = unsafe { WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, s, None, None, None) };
    if len == 0 {
        return Err(WinError::from_thread());
    }
    let mut buf = vec![0u8; len.cast_unsigned() as usize + 1]; // ensure that we definitely have a terminating null character
    unsafe { WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, s, Some(&mut buf), None, None) };
    match CStr::from_bytes_until_nul(buf.as_slice()) {
        Ok(c_str) => Ok(c_str.to_owned()),
        Err(_) => Err(WinError::from(E_STRING_NOT_NULL_TERMINATED)),
    }
}

pub(crate) fn copy_from_utf8_bytes(s: &[u8]) -> anyhow::Result<HSTRING> {
    let len = unsafe { MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, s, None) };
    anyhow::ensure!(len != 0, WinError::from_thread());
    let mut buf = vec![0u16; len.cast_unsigned() as usize + 1];
    let len = unsafe { MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, s, Some(&mut buf)) };
    anyhow::ensure!(len != 0, WinError::from_thread());
    Ok(HSTRING::from_wide(&buf[..len.cast_unsigned() as usize]))
}

pub(crate) fn copy_from_utf8_string(s: &BorrowedStrPtr) -> WinResult<HSTRING> {
    s.as_optional_cstr()
        .ok_or_else(|| WinError::from(E_POINTER))
        .and_then(|c_str| c_str.to_str().map_err(|_| WinError::from(ERROR_NO_UNICODE_TRANSLATION)))
        .map(HSTRING::from)
}
