use std::ffi::{CStr, CString};

use windows::Win32::{
    Foundation::E_STRING_NOT_NULL_TERMINATED,
    Globalization::{CP_UTF8, WC_ERR_INVALID_CHARS, WideCharToMultiByte},
};

#[allow(clippy::cast_sign_loss)]
pub(crate) fn copy_from_wide_string(s: &[u16]) -> windows::core::Result<CString> {
    let len = unsafe { WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, s, None, None, None) };
    if len == 0 {
        return Err(windows::core::Error::from_thread());
    }
    let mut buf = vec![0u8; len as usize + 1]; // ensure that we definitely have a terminating null character
    unsafe { WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, s, Some(buf.as_mut_slice()), None, None) };
    match CStr::from_bytes_until_nul(buf.as_slice()) {
        Ok(c_str) => Ok(c_str.to_owned()),
        Err(_) => Err(windows::core::Error::from(E_STRING_NOT_NULL_TERMINATED)),
    }
}
