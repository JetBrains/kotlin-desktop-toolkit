use anyhow::Context;
use objc2::rc::Retained;
use objc2_foundation::NSString;

use crate::{
    common::{BorrowedStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

#[unsafe(no_mangle)]
pub extern "C" fn string_drop(mut str_ptr: RustAllocatedStrPtr) {
    ffi_boundary("string_drop", || {
        str_ptr.deallocate();
        Ok(())
    });
}

pub(crate) fn copy_to_ns_string(s: &BorrowedStrPtr) -> anyhow::Result<Retained<NSString>> {
    let ptr = s.as_non_null().context("Null pointer")?;
    unsafe { NSString::stringWithUTF8String(ptr) }.context("stringWithUTF8String failed")
}

// Be aware, now you have to release this memory at some point
pub(crate) fn copy_to_c_string(ns_string: &NSString) -> anyhow::Result<RustAllocatedStrPtr> {
    let data: *const u8 = ns_string.UTF8String().cast();
    let len = ns_string.length();
    let slice = unsafe { std::slice::from_raw_parts(data, len) };
    Ok(RustAllocatedStrPtr::allocate(slice)?)
}
