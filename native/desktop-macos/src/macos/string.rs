use std::ffi::{CStr, CString};

use objc2::rc::{AutoreleasePool, Retained};
use objc2_foundation::NSString;

use crate::{common::StrPtr, logger::ffi_boundary};

#[unsafe(no_mangle)]
pub extern "C" fn string_drop(str_ptr: StrPtr) {
    ffi_boundary("string_drop", || {
        let s = unsafe {
            assert!(!str_ptr.is_null());
            CString::from_raw(str_ptr)
        };
        std::mem::drop(s);
        Ok(())
    });
}

pub(crate) fn copy_to_ns_string(str_ptr: StrPtr) -> anyhow::Result<Retained<NSString>> {
    let s: &str = unsafe { CStr::from_ptr(str_ptr) }.to_str()?;
    Ok(NSString::from_str(s))
}

// Be aware, now you have to release this memory at some point
pub(crate) fn copy_to_c_string(ns_string: &NSString, pool: AutoreleasePool) -> anyhow::Result<StrPtr> {
    let s: &str = unsafe { ns_string.to_str(pool) };
    Ok(CString::new(s)?.into_raw())
}
