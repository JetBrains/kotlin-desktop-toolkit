use desktop_common::{ffi_utils::BorrowedStrPtr, logger::ffi_boundary};
use objc2::rc::Retained;
use objc2_app_kit::NSPasteboard;

use super::string::copy_to_ns_string;

fn default_clipboard() -> Retained<NSPasteboard> {
    unsafe { NSPasteboard::generalPasteboard() }
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_clear() {
    ffi_boundary("pasteboard_clear", || {
        let clipboard = default_clipboard();
        unsafe {
            clipboard.clearContents();
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_write_text(text: BorrowedStrPtr) {
    ffi_boundary("pasteboard_clear", || {
        let _clipboard = default_clipboard();
        let _text = copy_to_ns_string(&text)?;
        //        clipboard
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use test_log::test;

    use log::info;
    use objc2::{rc::Retained, runtime::ProtocolObject};
    use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardType, NSPasteboardTypeString, NSURLNSPasteboardSupport};
    use objc2_foundation::{NSArray, NSObjectNSComparisonMethods, NSString, NSURL};

    fn default_clipboard() -> Retained<NSPasteboard> {
        //        return unsafe { NSPasteboard::generalPasteboard() };
        unsafe { NSPasteboard::pasteboardWithUniqueName() }
    }

    #[test]
    fn test_clipboard_can_store_and_return_string() {
        let clipboard = default_clipboard();
        let original_string = NSString::from_str("Hello");
        unsafe {
            clipboard.clearContents();
            clipboard.setString_forType(&original_string, NSPasteboardTypeString);
            let types = clipboard.types();
            assert!(types.is_some_and(|types| types.doesContain(NSPasteboardTypeString)));
            let string_from_clipboard = clipboard.stringForType(NSPasteboardTypeString);
            assert_eq!(Some(original_string), string_from_clipboard);
        }
    }

    #[test]
    fn test_empty_clipboard_doesnt_contain_string() {
        let clipboard = default_clipboard();
        unsafe {
            clipboard.clearContents();
            let string_from_clipboard = clipboard.stringForType(NSPasteboardTypeString);
            assert_eq!(None, string_from_clipboard);
        }
    }

    #[test]
    fn test_store_custom_type() {
        let clipboard = default_clipboard();
        let metadata_string = NSString::from_str("some metadata");
        let my_pasteboard_type: Retained<NSPasteboardType> = NSString::from_str("org.jetbrains.kdt.meta-string");
        unsafe {
            clipboard.clearContents();
            let change_count = clipboard.declareTypes_owner(&NSArray::from_slice(&[&*my_pasteboard_type]), None);
            // when we declare type it immediately shows in clipboard types
            // after clearContents it's gone
            info!("types1: {:?}", clipboard.types());
            info!("change: {:?}", change_count);
            clipboard.setString_forType(&metadata_string, &my_pasteboard_type);
            info!("types2: {:?}", clipboard.types());
            let string_from_clipboard = clipboard.stringForType(NSPasteboardTypeString);
            assert_eq!(None, string_from_clipboard);
            let meta_string_from_clipboard = clipboard.stringForType(&my_pasteboard_type);
            assert_eq!(Some(metadata_string), meta_string_from_clipboard);
            info!("clipboard items: {:?}", clipboard.pasteboardItems());
        }
    }

    #[test]
    fn test_store_custom_type_with_string() {
        let clipboard = default_clipboard();
        let metadata_string = NSString::from_str("some metadata");
        let original_string = NSString::from_str("Hello");
        let my_pasteboard_type: Retained<NSPasteboardType> = NSString::from_str("org.jetbrains.kdt.meta-string");
        unsafe {
            clipboard.clearContents();
            let change_count = clipboard.declareTypes_owner(&NSArray::from_slice(&[&*my_pasteboard_type]), None);
            clipboard.clearContents();
            info!("types1: {:?}", clipboard.types());
            info!("change: {:?}", change_count);
            clipboard.setString_forType(&metadata_string, &my_pasteboard_type);
            clipboard.setString_forType(&original_string, NSPasteboardTypeString);
            info!("types2: {:?}", clipboard.types());
            let string_from_clipboard = clipboard.stringForType(NSPasteboardTypeString);
            assert_eq!(Some(original_string), string_from_clipboard);
            let meta_string_from_clipboard = clipboard.stringForType(&my_pasteboard_type);
            assert_eq!(Some(metadata_string), meta_string_from_clipboard);
        }
    }

    #[test]
    fn test_store_two_files() {
        let general = default_clipboard();
        unsafe {
            general.clearContents();
            let url1 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("mouse.rs").to_str().unwrap()));
            let url2 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("string.rs").to_str().unwrap()));
            //            url.writeToPasteboard(&general); // apparently doesn't work
            general.writeObjects(&NSArray::from_slice(&[
                ProtocolObject::from_ref(&*url1),
                ProtocolObject::from_ref(&*url2),
            ]));
            let types = general.types();
            info!("types: {:?}", types);
            assert_eq!(Some(url1), NSURL::URLFromPasteboard(&general));
            assert_eq!(Some(2), general.pasteboardItems().map(|items| items.count()));
        }
    }

    fn get_source_file(filename: &str) -> PathBuf {
        let mut path = std::path::absolute(file!()).unwrap();
        path.pop();
        path.push(filename);
        path
    }

    #[test]
    fn test_store_two_files_together_with_string() {
        let general = default_clipboard();
        let original_string = NSString::from_str("Hello");
        unsafe {
            general.clearContents();
            let url1 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("mouse.rs").to_str().unwrap()));
            let url2 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("string.rs").to_str().unwrap()));
            // url.writeToPasteboard(&general); // apparently doesn't work
            general.writeObjects(&NSArray::from_slice(&[
                ProtocolObject::from_ref(&*url1),
                ProtocolObject::from_ref(&*url2),
            ]));
            general.setString_forType(&original_string, NSPasteboardTypeString);
            let types = general.types();
            info!("types: {:?}", types);
            assert_eq!(Some(url1), NSURL::URLFromPasteboard(&general));
            let items = general.pasteboardItems().unwrap().to_vec();
            assert_eq!(2, items.len());
        }
    }

    #[test]
    fn test_create_pasteboard_item() {
        let general = default_clipboard();
        let item = unsafe { NSPasteboardItem::new() };
        let original_string = NSString::from_str("Hello");
        unsafe {
            general.clearContents();
            let url1 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("mouse.rs").to_str().unwrap()));
            item.setString_forType(&original_string, NSPasteboardTypeString);
            let result = general.writeObjects(&NSArray::from_slice(&[
                ProtocolObject::from_ref(&*url1),
                ProtocolObject::from_ref(&*item),
            ]));
            assert!(result);
        }
    }

    #[test]
    fn test_clear_is_required_before_write_objects() {
        let general = default_clipboard();
        unsafe {
            general.clearContents();
            let item1 = NSPasteboardItem::new();
            item1.setString_forType(&NSString::from_str("Hello"), NSPasteboardTypeString);
            let result1 = general.writeObjects(&NSArray::from_slice(&[ProtocolObject::from_ref(&*item1)]));
            assert!(result1);

            let item2 = NSPasteboardItem::new();
            item2.setString_forType(&NSString::from_str("World"), NSPasteboardTypeString);
            let result2 = general.writeObjects(&NSArray::from_slice(&[ProtocolObject::from_ref(&*item2)]));
            // this might be false if the application lost clipboard ownership
            assert!(result2);
        }
    }
}
