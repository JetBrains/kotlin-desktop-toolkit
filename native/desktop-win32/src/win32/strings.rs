use desktop_common::ffi_utils::BorrowedStrPtr;
use std::ffi::CString;

use anyhow::Context;
use windows::Win32::{
    Foundation::{E_POINTER, ERROR_NO_UNICODE_TRANSLATION},
    Globalization::{CP_UTF8, MB_ERR_INVALID_CHARS, MultiByteToWideChar, WC_ERR_INVALID_CHARS, WideCharToMultiByte},
};
use windows_core::{Error as WinError, HSTRING};

pub(crate) fn copy_from_wide_string(s: &[u16]) -> anyhow::Result<CString> {
    let len = unsafe { WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, s, None, None, None) };
    anyhow::ensure!(len != 0, WinError::from_thread());
    let mut buf = vec![0u8; len.cast_unsigned() as usize + 1]; // ensure that we definitely have a terminating null character
    let len = unsafe { WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, s, Some(&mut buf), None, None) };
    anyhow::ensure!(len != 0, WinError::from_thread());
    CString::from_vec_with_nul(buf).context("failed to create a CString from bytes")
}

pub(crate) fn copy_from_utf8_bytes(s: &[u8]) -> anyhow::Result<HSTRING> {
    let len = unsafe { MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, s, None) };
    anyhow::ensure!(len != 0, WinError::from_thread());
    let mut buf = vec![0u16; len.cast_unsigned() as usize + 1];
    let len = unsafe { MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, s, Some(&mut buf)) };
    anyhow::ensure!(len != 0, WinError::from_thread());
    Ok(HSTRING::from_wide(&buf[..len.cast_unsigned() as usize]))
}

pub(crate) fn copy_from_utf8_string(s: &BorrowedStrPtr) -> anyhow::Result<HSTRING> {
    let c_str = s.as_optional_cstr().with_context(|| WinError::from(E_POINTER))?;
    let str = c_str.to_str().with_context(|| WinError::from(ERROR_NO_UNICODE_TRANSLATION))?;
    Ok(HSTRING::from(str))
}
