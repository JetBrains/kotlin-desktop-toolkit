use crate::macos::string::copy_to_c_string;
use desktop_common::ffi_utils::AutoDropArray;
use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use objc2_foundation::{MainThreadMarker, NSString};
use std::ffi::c_void;

#[allow(unused_doc_comments)]
/// cbindgen:ignore
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *const c_void;
    // Note: TISGetInputSourceProperty returns a borrowed reference, NOT an owned one
    fn TISGetInputSourceProperty(inputSource: *const c_void, propertyKey: *const c_void) -> *const c_void;
    fn TISCreateInputSourceList(properties: *const c_void, include_all_installed: bool) -> *const c_void;
    fn TISSelectInputSource(inputSource: *const c_void) -> i32;
    #[allow(dead_code)]
    static kTISPropertyUnicodeKeyLayoutData: *const c_void;
    static kTISPropertyInputSourceID: *const c_void;
    #[allow(dead_code)]
    static kTISPropertyLocalizedName: *const c_void;
    static kTISPropertyInputSourceIsASCIICapable: *const c_void;
}

#[allow(unused_doc_comments)]
/// cbindgen:ignore
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
    fn CFBooleanGetValue(boolean: *const c_void) -> bool;
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_source_current() -> RustAllocatedStrPtr {
    ffi_boundary("text_input_source_current", || {
        let _mtm = MainThreadMarker::new().unwrap();
        unsafe {
            let input_source = TISCopyCurrentKeyboardLayoutInputSource();
            if input_source.is_null() {
                log::warn!("Can't get current keyboard layout");
                return Ok(RustAllocatedStrPtr::null());
            }

            let source_id_ptr = TISGetInputSourceProperty(input_source, kTISPropertyInputSourceID);

            let result = if source_id_ptr.is_null() {
                Ok(RustAllocatedStrPtr::null())
            } else {
                // source_id is a CFStringRef (borrowed), toll-free bridged to NSString
                let ns_string: &NSString = &*source_id_ptr.cast::<NSString>();
                copy_to_c_string(ns_string)
            };

            // Release the input source we got from TISCopyCurrentKeyboardLayoutInputSource
            CFRelease(input_source);

            result
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_source_is_ascii_capable(source_id: BorrowedStrPtr) -> bool {
    ffi_boundary("text_input_source_is_ascii_capable", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let source_id_str = source_id.as_str()?;
        unsafe {
            let source_list = TISCreateInputSourceList(std::ptr::null(), true);
            if source_list.is_null() {
                return Ok(false);
            }

            #[allow(clippy::cast_sign_loss)]
            let count = CFArrayGetCount(source_list) as usize;
            let mut result = false;

            for i in 0..count {
                let input_source = CFArrayGetValueAtIndex(source_list, i as isize);
                let id_ptr = TISGetInputSourceProperty(input_source, kTISPropertyInputSourceID);
                if !id_ptr.is_null() {
                    let ns_string: &NSString = &*id_ptr.cast::<NSString>();
                    if ns_string.to_string() == source_id_str {
                        let is_ascii_capable_ptr = TISGetInputSourceProperty(input_source, kTISPropertyInputSourceIsASCIICapable);
                        if !is_ascii_capable_ptr.is_null() {
                            result = CFBooleanGetValue(is_ascii_capable_ptr);
                        }
                        break;
                    }
                }
            }

            CFRelease(source_list);
            Ok(result)
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_source_list() -> AutoDropArray<RustAllocatedStrPtr> {
    ffi_boundary("text_input_source_list", || {
        unsafe {
            let source_list = TISCreateInputSourceList(std::ptr::null(), false);
            if source_list.is_null() {
                return Ok(AutoDropArray::new(Box::new([])));
            }

            #[allow(clippy::cast_sign_loss)]
            let count = CFArrayGetCount(source_list) as usize;

            if count == 0 {
                CFRelease(source_list);
                return Ok(AutoDropArray::new(Box::new([])));
            }

            let mut source_ids: Vec<RustAllocatedStrPtr> = Vec::with_capacity(count);

            for i in 0..count {
                let input_source = CFArrayGetValueAtIndex(source_list, i as isize);
                let source_id_ptr = TISGetInputSourceProperty(input_source, kTISPropertyInputSourceID);
                if !source_id_ptr.is_null() {
                    // source_id is a CFStringRef (borrowed), toll-free bridged to NSString
                    let ns_string: &NSString = &*source_id_ptr.cast::<NSString>();
                    source_ids.push(copy_to_c_string(ns_string)?);
                }
            }

            CFRelease(source_list);

            Ok(AutoDropArray::new(source_ids.into_boxed_slice()))
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_source_select(source_id: BorrowedStrPtr) -> bool {
    ffi_boundary("text_input_source_select", || {
        let source_id_str = source_id.as_str()?;
        unsafe {
            let source_list = TISCreateInputSourceList(std::ptr::null(), true);
            if source_list.is_null() {
                return Ok(false);
            }

            #[allow(clippy::cast_sign_loss)]
            let count = CFArrayGetCount(source_list) as usize;
            let mut result = false;

            for i in 0..count {
                let input_source = CFArrayGetValueAtIndex(source_list, i as isize);
                let prop_ptr = TISGetInputSourceProperty(input_source, kTISPropertyInputSourceID);
                if !prop_ptr.is_null() {
                    let ns_string: &NSString = &*prop_ptr.cast::<NSString>();
                    if ns_string.to_string() == source_id_str {
                        let status = TISSelectInputSource(input_source);
                        result = status == 0; // noErr
                        break;
                    }
                }
            }

            CFRelease(source_list);
            Ok(result)
        }
    })
}
