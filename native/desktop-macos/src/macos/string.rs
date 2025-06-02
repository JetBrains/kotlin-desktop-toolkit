use std::ffi::CStr;

use anyhow::Context;
use objc2::rc::Retained;
use objc2_foundation::NSString;

use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

#[unsafe(no_mangle)]
pub extern "C" fn string_drop(mut str_ptr: RustAllocatedStrPtr) {
    ffi_boundary("string_drop", || {
        str_ptr.deallocate();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn string_array_drop(str_array: AutoDropArray<RustAllocatedStrPtr>) {
    ffi_boundary("string_array_drop", || {
        drop(str_array);
        Ok(())
    });
}

pub(crate) fn borrow_ns_string(s: &NSString) -> BorrowedStrPtr<'_> {
    let c_str = unsafe { CStr::from_ptr(s.UTF8String()) };
    BorrowedStrPtr::new(c_str)
}

pub(crate) fn copy_to_ns_string(s: &BorrowedStrPtr) -> anyhow::Result<Retained<NSString>> {
    let ptr = s.as_non_null().context("Null pointer")?;
    unsafe { NSString::stringWithUTF8String(ptr) }.context("stringWithUTF8String failed")
}

// Be aware, now you have to release this memory at some point
pub(crate) fn copy_to_c_string(ns_string: &NSString) -> anyhow::Result<RustAllocatedStrPtr> {
    let c_str = unsafe { CStr::from_ptr(ns_string.UTF8String()) };
    Ok(RustAllocatedStrPtr::allocate(c_str.to_bytes())?)
}

#[cfg(test)]
mod tests {
    use objc2_foundation::{NSString, ns_string};

    use crate::macos::string::copy_to_c_string;

    use super::{borrow_ns_string, copy_to_ns_string};

    #[test]
    fn test_string_conversion_is_reversable() {
        let ns_string = ns_string!("Hello😃World\nOne More Line");
        let borrowed_c_str = borrow_ns_string(ns_string);
        let new_ns_string = copy_to_ns_string(&borrowed_c_str).unwrap();
        assert_eq!(ns_string, &*new_ns_string);
    }

    #[test]
    fn test_string_conversion_with_copy_is_reversable() {
        let ns_string = ns_string!("Hello😃World\nOne More Line");
        let c_str = copy_to_c_string(ns_string).unwrap();
        let str = c_str.as_str().unwrap();
        let new_ns_string = NSString::from_str(str);
        assert_eq!(ns_string, &*new_ns_string);
    }
}
