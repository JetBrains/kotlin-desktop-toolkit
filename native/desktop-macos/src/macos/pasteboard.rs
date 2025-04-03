use std::sync::Mutex;

use anyhow::Context;
use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedArray, BorrowedStrPtr, RustAllocatedStrPtr},
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;
use objc2::{
    ClassType,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardURLReadingFileURLsOnlyKey, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSDictionary, NSMutableArray, NSNumber, NSURL};

use super::string::{copy_to_c_string, copy_to_ns_string};

static GENERAL_PASTEBOARD_SHARED_TOKEN: Mutex<()> = Mutex::new(());

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum PasteboardType {
    Global,
    WithUniqueName,
}

fn with_pasteboard<R, F: FnOnce(&NSPasteboard) -> R>(pasteboard_type: PasteboardType, f: F) -> R {
    match pasteboard_type {
        PasteboardType::Global => {
            // We could get multiple refs to general clipboard with calling `NSPasteboard::generalPasteboard()`
            // from multiple threads. Though the NSPasteboard isn't thread safe class
            let _shared_token = GENERAL_PASTEBOARD_SHARED_TOKEN.lock();
            let pasteboard = unsafe { NSPasteboard::generalPasteboard() };
            f(&pasteboard)
        }
        PasteboardType::WithUniqueName => {
            let pasteboard = unsafe { NSPasteboard::pasteboardWithUniqueName() };
            f(&pasteboard)
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_clear() -> isize {
    ffi_boundary("pasteboard_clear", || {
        let result = with_pasteboard(PasteboardType::Global, |pasteboard| unsafe { pasteboard.clearContents() });
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
        with_pasteboard(PasteboardType::Global, |pasteboard| {
            debug!("pasteboard_write_objects: {:?}", items);
            let objects = copy_to_objects(&items)?;
            Ok(unsafe { pasteboard.writeObjects(&objects) })
        })
    })
}

#[repr(C)]
pub struct PasteboardContentResult {
    items: AutoDropArray<RustAllocatedStrPtr>,
}

impl PanicDefault for PasteboardContentResult {
    fn default() -> Self {
        Self {
            items: AutoDropArray::new(Box::new([])),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_read_items_of_type(uniform_type_identifier: BorrowedStrPtr) -> PasteboardContentResult {
    ffi_boundary("pasteboard_read_content_for_type", || {
        with_pasteboard(PasteboardType::Global, |pasteboard| {
            let uti = copy_to_ns_string(&uniform_type_identifier)?;
            let items = unsafe { pasteboard.pasteboardItems() }.context("Can't retrieve items")?;
            let items: anyhow::Result<Box<[_]>> = items
                .iter()
                .filter_map(|item| unsafe { item.stringForType(&uti) })
                .map(|item_str| copy_to_c_string(&item_str))
                .collect();
            Ok(PasteboardContentResult {
                items: AutoDropArray::new(items?),
            })
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_read_file_items() -> PasteboardContentResult {
    ffi_boundary("pasteboard_read_file_items", || {
        with_pasteboard(PasteboardType::Global, |pasteboard| {
            let class_array = NSArray::from_slice(&[NSURL::class()]);

            let options = NSDictionary::from_slices(
                &[unsafe { NSPasteboardURLReadingFileURLsOnlyKey }],
                &[&*Retained::<AnyObject>::from(NSNumber::numberWithBool(true))],
            );
            let urls = unsafe { pasteboard.readObjectsForClasses_options(&class_array, Some(&*options)) }.context("No items")?;

            let urls: anyhow::Result<Box<_>> = urls
                .iter()
                .filter_map(|url| {
                    let url = url.downcast::<NSURL>().expect("It must be NSURL");
                    unsafe { url.filePathURL() }
                })
                .map(|url_ns_str| copy_to_c_string(&*unsafe { url_ns_str.path().expect("Expected path here") }))
                .collect();

            Ok(PasteboardContentResult {
                items: AutoDropArray::new(urls?),
            })
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_content_drop(content: PasteboardContentResult) {
    ffi_boundary("pasteboard_content_drop", || {
        std::mem::drop(content);
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use test_log::test;

    use log::info;
    use objc2::{rc::Retained, runtime::ProtocolObject};
    use objc2_app_kit::{NSPasteboardItem, NSPasteboardType, NSPasteboardTypeString, NSURLNSPasteboardSupport};
    use objc2_foundation::{NSArray, NSObjectNSComparisonMethods, NSString, NSURL};

    use crate::macos::pasteboard::{PasteboardType, with_pasteboard};

    #[test]
    fn test_pasteboard_can_store_and_return_string() {
        with_pasteboard(PasteboardType::WithUniqueName, |pasteboard| {
            let original_string = NSString::from_str("Hello");
            unsafe {
                info!("NSPasteboardTypeString: {:?}", NSPasteboardTypeString);
                pasteboard.clearContents();
                pasteboard.setString_forType(&original_string, NSPasteboardTypeString);
                let types = pasteboard.types();
                assert!(types.is_some_and(|types| types.doesContain(NSPasteboardTypeString)));
                let string_from_pasteboard = pasteboard.stringForType(NSPasteboardTypeString);
                assert_eq!(Some(original_string), string_from_pasteboard);
            }
        });
    }

    #[test]
    fn test_empty_pasteboard_doesnt_contain_string() {
        with_pasteboard(PasteboardType::WithUniqueName, |pasteboard| {
            unsafe {
                pasteboard.clearContents();
            }
            let string_from_pasteboard = unsafe { pasteboard.stringForType(NSPasteboardTypeString) };
            assert_eq!(None, string_from_pasteboard);
        });
    }

    #[test]
    fn test_store_custom_type() {
        with_pasteboard(PasteboardType::WithUniqueName, |pasteboard| {
            let metadata_string = NSString::from_str("some metadata");
            let my_pasteboard_type: Retained<NSPasteboardType> = NSString::from_str("org.jetbrains.kdt.meta-string");
            unsafe {
                pasteboard.clearContents();
                let change_count = pasteboard.declareTypes_owner(&NSArray::from_slice(&[&*my_pasteboard_type]), None);
                // when we declare type it immediately shows in pasteboard types
                // after clearContents it's gone
                info!("types1: {:?}", pasteboard.types());
                info!("change: {:?}", change_count);
                pasteboard.setString_forType(&metadata_string, &my_pasteboard_type);
                info!("types2: {:?}", pasteboard.types());
                let string_from_pasteboard = pasteboard.stringForType(NSPasteboardTypeString);
                assert_eq!(None, string_from_pasteboard);
                let meta_string_from_pasteboard = pasteboard.stringForType(&my_pasteboard_type);
                assert_eq!(Some(metadata_string), meta_string_from_pasteboard);
                info!("pasteboard items: {:?}", pasteboard.pasteboardItems());
            }
        });
    }

    #[test]
    fn test_store_custom_type_with_string() {
        with_pasteboard(PasteboardType::WithUniqueName, |pasteboard| {
            let metadata_string = NSString::from_str("some metadata");
            let original_string = NSString::from_str("Hello");
            let my_pasteboard_type: Retained<NSPasteboardType> = NSString::from_str("org.jetbrains.kdt.meta-string");
            unsafe {
                pasteboard.clearContents();
                let change_count = pasteboard.declareTypes_owner(&NSArray::from_slice(&[&*my_pasteboard_type]), None);
                pasteboard.clearContents();
                info!("types1: {:?}", pasteboard.types());
                info!("change: {:?}", change_count);
                pasteboard.setString_forType(&metadata_string, &my_pasteboard_type);
                pasteboard.setString_forType(&original_string, NSPasteboardTypeString);
                info!("types2: {:?}", pasteboard.types());
                let string_from_pasteboard = pasteboard.stringForType(NSPasteboardTypeString);
                assert_eq!(Some(original_string), string_from_pasteboard);
                let meta_string_from_pasteboard = pasteboard.stringForType(&my_pasteboard_type);
                assert_eq!(Some(metadata_string), meta_string_from_pasteboard);
            }
        });
    }

    #[test]
    fn test_store_two_files() {
        with_pasteboard(PasteboardType::WithUniqueName, |general| {
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
        with_pasteboard(PasteboardType::WithUniqueName, |general| {
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
        with_pasteboard(PasteboardType::WithUniqueName, |general| {
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
        with_pasteboard(PasteboardType::WithUniqueName, |general| {
            unsafe {
                general.clearContents();
                let item1 = NSPasteboardItem::new();
                item1.setString_forType(&NSString::from_str("Hello"), NSPasteboardTypeString);
                let result1 = general.writeObjects(&NSArray::from_slice(&[ProtocolObject::from_ref(&*item1)]));
                assert!(result1);

                let item2 = NSPasteboardItem::new();
                item2.setString_forType(&NSString::from_str("World"), NSPasteboardTypeString);
                let result2 = general.writeObjects(&NSArray::from_slice(&[ProtocolObject::from_ref(&*item2)]));
                // this might be false if the application lost pasteboard ownership
                assert!(result2);
            }
        });
    }
}
