use crate::macos::string::copy_to_c_string;
use desktop_common::ffi_utils::AutoDropArray;
use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use objc2_foundation::{MainThreadMarker, NSDictionary, NSString};
use std::ffi::c_void;

#[allow(unused_doc_comments)]
/// cbindgen:ignore
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardInputSource() -> *const c_void;
    // Note: TISGetInputSourceProperty returns a borrowed reference, NOT an owned one
    fn TISGetInputSourceProperty(inputSource: *const c_void, propertyKey: *const c_void) -> *const c_void;
    fn TISCreateInputSourceList(properties: *const c_void, include_all_installed: bool) -> *const c_void;
    fn TISSelectInputSource(inputSource: *const c_void) -> i32;
    fn TISEnableInputSource(inputSource: *const c_void) -> i32;
    fn TISDisableInputSource(inputSource: *const c_void) -> i32;
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
    #[allow(dead_code)]
    fn CFRetain(cf: *const c_void) -> *const c_void;
    fn CFArrayGetCount(array: *const c_void) -> isize;
    fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
    fn CFBooleanGetValue(boolean: *const c_void) -> bool;
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_source_current() -> RustAllocatedStrPtr {
    ffi_boundary("text_input_source_current", || {
        let _mtm = MainThreadMarker::new().unwrap();
        unsafe {
            let input_source = TISCopyCurrentKeyboardInputSource();
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
pub extern "C" fn text_input_source_select(source_id: BorrowedStrPtr) -> bool {
    ffi_boundary("text_input_source_select", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let source_id_str = source_id.as_str()?;
        unsafe {
            let source_id_ns = NSString::from_str(source_id_str);
            let key: &NSString = &*(kTISPropertyInputSourceID as *const NSString);
            let search_params = NSDictionary::from_slices(&[key], &[&*source_id_ns]);

            let dict_ptr: *const NSDictionary<NSString, NSString> = &*search_params;
            let source_list = TISCreateInputSourceList(dict_ptr.cast(), false);
            if source_list.is_null() {
                return Ok(false);
            }

            let count = CFArrayGetCount(source_list);
            if count == 0 {
                CFRelease(source_list);
                return Ok(false);
            }

            let input_source = CFArrayGetValueAtIndex(source_list, 0);
            let status = TISSelectInputSource(input_source);
            let result = status == 0; // noErr

            CFRelease(source_list);
            Ok(result)
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_source_list(include_all: bool) -> AutoDropArray<RustAllocatedStrPtr> {
    ffi_boundary("text_input_source_list", || {
        let _mtm = MainThreadMarker::new().unwrap();
        unsafe {
            let source_list = TISCreateInputSourceList(std::ptr::null(), include_all);
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

// NSDictionary* searchParam = @{ (NSString*)kTISPropertyInputSourceID : layoutId };
// CFArrayRef sources = TISCreateInputSourceList((CFDictionaryRef)searchParam, YES);
//
// if (CFArrayGetCount(sources) == 0) {
// NSLog(@"failed to set keyboard layout %@ enabled state: no such layout", layoutId);
// CFRelease(sources);
// return;
// }
//
// TISInputSourceRef source = (TISInputSourceRef)CFArrayGetValueAtIndex(sources, 0);
// OSStatus status = enabled ? TISEnableInputSource(source) : TISDisableInputSource(source);
// if (status == noErr) {
// success = true;
// } else {
// NSLog(@"failed to set keyboard layout %@ enabled state, error code %d", layoutId, status);
// }
//
// CFRelease(sources);

#[unsafe(no_mangle)]
pub extern "C" fn text_input_source_set_enable(source_id: BorrowedStrPtr, enabled: bool) -> bool {
    ffi_boundary("text_input_source_set_enable", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let source_id_str = source_id.as_str()?;
        unsafe {
            let source_id_ns = NSString::from_str(source_id_str);
            let key: &NSString = &*(kTISPropertyInputSourceID as *const NSString);
            let search_params = NSDictionary::from_slices(&[key], &[&*source_id_ns]);

            let dict_ptr: *const NSDictionary<NSString, NSString> = &*search_params;
            let source_list = TISCreateInputSourceList(dict_ptr.cast(), true);
            if source_list.is_null() {
                return Ok(false);
            }

            let count = CFArrayGetCount(source_list);
            if count == 0 {
                CFRelease(source_list);
                return Ok(false);
            }

            let input_source = CFArrayGetValueAtIndex(source_list, 0);
            let status = if enabled {
                TISEnableInputSource(input_source)
            } else {
                TISDisableInputSource(input_source)
            };
            let result = status == 0; // noErr

            CFRelease(source_list);
            Ok(result)
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
