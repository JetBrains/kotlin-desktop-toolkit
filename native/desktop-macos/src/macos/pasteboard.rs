use anyhow::Context;
use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedStrPtr},
    logger::ffi_boundary,
};
use log::debug;
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSMutableArray, NSURL};

use super::string::copy_to_ns_string;

fn default_clipboard() -> Retained<NSPasteboard> {
    unsafe { NSPasteboard::generalPasteboard() }
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_clear() -> isize {
    ffi_boundary("pasteboard_clear", || {
        debug!("pasteboard_clear!");
        let clipboard = default_clipboard();
        let result = unsafe { clipboard.clearContents() };
        Ok(result)
    })
}

#[repr(C)]
#[derive(Debug)]
pub struct CombinedItemElement<'a> {
    // todo: later we need to support elements with binary payload
    // todo: and elements with lazy data providers
    pub uniform_type_identifier: BorrowedStrPtr<'a>,
    pub content: BorrowedStrPtr<'a>,
}

#[repr(C)]
#[derive(Debug)]
pub enum PasteboardItem<'a> {
    // NSURL
    URLItem {
        url: BorrowedStrPtr<'a>,
    },
    // todo we could add some more NS* classes that implements NSPasteboardWriting protocol
    CombinedItem {
        elements: BorrowedArray<'a, CombinedItemElement<'a>>,
    },
}

fn copy_to_objects(items: &BorrowedArray<PasteboardItem>) -> anyhow::Result<Retained<NSArray<ProtocolObject<dyn NSPasteboardWriting>>>> {
    let items = items.as_slice()?;
    let array = unsafe { NSMutableArray::<ProtocolObject<dyn NSPasteboardWriting>>::arrayWithCapacity(items.len()) };
    for item in items {
        match item {
            PasteboardItem::URLItem { url } => {
                let url = copy_to_ns_string(url)?;
                let ns_url = unsafe { NSURL::URLWithString(&url) }.with_context(|| format!("Malformed URL: {url:?}"))?;
                debug!("is file url: {:?}", unsafe { ns_url.isFileURL() });
                array.addObject(&ProtocolObject::from_retained(ns_url));
            }
            PasteboardItem::CombinedItem { elements } => {
                let elements = elements.as_slice()?;
                let item = unsafe { NSPasteboardItem::new() };
                for element in elements {
                    let uti = copy_to_ns_string(&element.uniform_type_identifier)?;
                    let content = copy_to_ns_string(&element.content)?;
                    unsafe {
                        item.setString_forType(&content, &uti);
                    }
                }
                array.addObject(&ProtocolObject::from_retained(item));
            }
        }
    }
    Ok(array.into_super())
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_write_objects(items: BorrowedArray<PasteboardItem>) -> bool {
    ffi_boundary("pasteboard_write_objects", || {
        debug!("pasteboard_write_objects: {:?}", items);
        let clipboard = default_clipboard();
        let objects = copy_to_objects(&items)?;
        Ok(unsafe { clipboard.writeObjects(&objects) })
    })
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Mutex};
    use test_log::test;

    use log::info;
    use objc2::{rc::Retained, runtime::ProtocolObject};
    use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardType, NSPasteboardTypeString, NSURLNSPasteboardSupport};
    use objc2_foundation::{NSArray, NSObjectNSComparisonMethods, NSString, NSURL};

    static GENERAL_CLIPBOARD_SHARED_TOKEN: Mutex<()> = Mutex::new(());

    fn with_clipboard<F: FnOnce(&NSPasteboard)>(f: F) {
        let use_global = true;
        if use_global {
            // We could get multiple refs to general clipboard with calling `NSPasteboard::generalPasteboard()`
            // from multiple threads. Though the NSPasteboard isn't thread safe class
            let _shared_token = GENERAL_CLIPBOARD_SHARED_TOKEN.lock();
            let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
            f(&pasteboard);
        } else {
            let pasteboard = unsafe { NSPasteboard::pasteboardWithUniqueName() };
            f(&pasteboard);
        }
    }

    #[test]
    fn test_clipboard_can_store_and_return_string() {
        with_clipboard(|clipboard| {
            let original_string = NSString::from_str("Hello");
            unsafe {
                info!("NSPasteboardTypeString: {:?}", NSPasteboardTypeString);
                clipboard.clearContents();
                clipboard.setString_forType(&original_string, NSPasteboardTypeString);
                let types = clipboard.types();
                assert!(types.is_some_and(|types| types.doesContain(NSPasteboardTypeString)));
                let string_from_clipboard = clipboard.stringForType(NSPasteboardTypeString);
                assert_eq!(Some(original_string), string_from_clipboard);
            }
        });
    }

    #[test]
    fn test_empty_clipboard_doesnt_contain_string() {
        with_clipboard(|clipboard| unsafe {
            clipboard.clearContents();
            let string_from_clipboard = clipboard.stringForType(NSPasteboardTypeString);
            assert_eq!(None, string_from_clipboard);
        });
    }

    #[test]
    fn test_store_custom_type() {
        with_clipboard(|clipboard| {
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
        });
    }

    #[test]
    fn test_store_custom_type_with_string() {
        with_clipboard(|clipboard| {
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
        });
    }

    #[test]
    fn test_store_two_files() {
        with_clipboard(|general| {
            unsafe {
                let url1 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("mouse.rs").to_str().unwrap()));
                let url2 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("string.rs").to_str().unwrap()));
                // url.writeToPasteboard(&general); // apparently doesn't work
                general.clearContents();
                let result = general.writeObjects(&NSArray::from_slice(&[
                    ProtocolObject::from_ref(&*url1),
                    ProtocolObject::from_ref(&*url2),
                ]));
                assert!(result);
                let types = general.types();
                info!("types: {:?}", types);
                assert_eq!(Some(url1), NSURL::URLFromPasteboard(general));
                assert_eq!(Some(2), general.pasteboardItems().map(|items| items.count()));
            }
        });
    }

    fn get_source_file(filename: &str) -> PathBuf {
        let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = crate_root.parent().unwrap();

        let current_file = workspace_root.join(file!());
        let source_file = current_file.parent().unwrap().join(filename);

        assert!(source_file.exists());
        source_file
    }

    #[test]
    fn test_store_two_files_together_with_string() {
        with_clipboard(|general| {
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
                assert_eq!(Some(url1), NSURL::URLFromPasteboard(general));
                let items = general.pasteboardItems().unwrap().to_vec();
                assert_eq!(2, items.len());
            }
        });
    }

    #[test]
    fn test_create_pasteboard_item() {
        with_clipboard(|general| {
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
        });
    }

    #[test]
    fn test_clear_is_required_before_write_objects() {
        with_clipboard(|general| {
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
        });
    }
}
