use desktop_common::ffi_utils::BorrowedStrPtr;
use std::ffi::CString;

use anyhow::Context;
use windows::Win32::Foundation::{E_POINTER, ERROR_NO_UNICODE_TRANSLATION};
use windows_core::{Error as WinError, HSTRING};

pub(crate) fn copy_from_wide_string(s: &[u16]) -> anyhow::Result<CString> {
    let str = String::from_utf16(s.strip_suffix(&[0]).unwrap_or(s)).with_context(|| WinError::from(ERROR_NO_UNICODE_TRANSLATION))?;
    CString::new(str).with_context(|| "failed to create a CString from wide chars")
}

pub(crate) fn copy_from_utf8_bytes(s: &[u8]) -> anyhow::Result<HSTRING> {
    let str = str::from_utf8(s.strip_suffix(&[0]).unwrap_or(s)).with_context(|| WinError::from(ERROR_NO_UNICODE_TRANSLATION))?;
    Ok(HSTRING::from(str))
}

pub(crate) fn copy_from_utf8_string(s: &BorrowedStrPtr) -> anyhow::Result<HSTRING> {
    let c_str = s.as_optional_cstr().with_context(|| WinError::from(E_POINTER))?;
    let str = c_str.to_str().with_context(|| WinError::from(ERROR_NO_UNICODE_TRANSLATION))?;
    Ok(HSTRING::from(str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_from_wide_string_with_nul() {
        let s = windows_core::w!("Hello world!\0");
        let c_str = copy_from_wide_string(unsafe { s.as_wide() }).unwrap();
        assert_eq!(c_str.to_str().unwrap(), "Hello world!");
    }

    #[test]
    fn copy_from_wide_string_without_nul() {
        let s = windows_core::w!("Hello world!");
        let c_str = copy_from_wide_string(unsafe { s.as_wide() }).unwrap();
        assert_eq!(c_str.to_str().unwrap(), "Hello world!");
    }

    #[test]
    fn copy_from_utf8_bytes_with_nul() {
        let hstr = copy_from_utf8_bytes(b"Hello world!\0").unwrap();
        assert_eq!(&hstr, "Hello world!");
    }

    #[test]
    fn copy_from_utf8_bytes_without_nul() {
        let hstr = copy_from_utf8_bytes(b"Hello world!").unwrap();
        assert_eq!(&hstr, "Hello world!");
    }

    #[test]
    fn copy_from_utf8_string_with_string() {
        let c_str = CString::new(b"Hello world!").expect("CString::new failed");
        let borrowed_str = BorrowedStrPtr::new(&c_str);
        let hstr = copy_from_utf8_string(&borrowed_str).unwrap();
        assert_eq!(&hstr, "Hello world!");
    }

    #[test]
    fn copy_from_utf8_string_with_null() {
        let borrowed_str = BorrowedStrPtr::null();
        let hstr_err = copy_from_utf8_string(&borrowed_str).unwrap_err();
        assert_eq!(hstr_err.downcast_ref::<WinError>().unwrap(), &WinError::from(E_POINTER));
    }
}
