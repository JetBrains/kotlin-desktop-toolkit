use std::ffi::c_void;

pub(crate) type StrPtr = *const std::ffi::c_char;
pub (crate) type ArraySize = i64;

#[repr(C)]
pub(crate) struct Array {
    arr: *const c_void,
    len: ArraySize,
}