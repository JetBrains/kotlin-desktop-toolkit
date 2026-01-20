use std::{ffi::CStr, sync::Mutex};

use super::{string::copy_to_ns_string, url::url_to_file_path_string};
use crate::macos::string::copy_to_ns_string_if_not_null;
use anyhow::Context;
use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedArray, BorrowedStrPtr},
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;
use objc2::{
    ClassType,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardURLReadingFileURLsOnlyKey, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSDictionary, NSMutableArray, NSNumber, NSString, NSURL};

/// cbindgen:ignore
static GENERAL_PASTEBOARD_SHARED_TOKEN: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone)]
enum PasteboardType {
    Global,
    #[allow(dead_code)] // Used only it tests
    WithUniqueName,
    WithName(Retained<NSString>),
}

fn with_pasteboard<R, F: FnOnce(&NSPasteboard) -> R>(pasteboard_type: &PasteboardType, f: F) -> R {
    match pasteboard_type {
        PasteboardType::Global => {
            // We could get multiple refs to general clipboard with calling `NSPasteboard::generalPasteboard()`
            // from multiple threads. Though the NSPasteboard isn't a thread-safe class
            let _shared_token = GENERAL_PASTEBOARD_SHARED_TOKEN.lock();
            let pasteboard = NSPasteboard::generalPasteboard();
            f(&pasteboard)
        }
        PasteboardType::WithName(pasteboard_name) => {
            // It's a bit more restrictive than it should be
            // We could have separate locks for separate clipboards
            // And maybe the lock should be reentrant
            let _shared_token = GENERAL_PASTEBOARD_SHARED_TOKEN.lock();
            let pasteboard = NSPasteboard::pasteboardWithName(pasteboard_name);
            f(&pasteboard)
        }
        PasteboardType::WithUniqueName => {
            let pasteboard = NSPasteboard::pasteboardWithUniqueName();
            f(&pasteboard)
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_clear() -> isize {
    ffi_boundary("pasteboard_clear", || {
        let result = with_pasteboard(&PasteboardType::Global, NSPasteboard::clearContents);
        Ok(result)
    })
}

#[repr(C)]
#[derive(Debug)]
pub struct CombinedItemElement<'a> {
    // todo: and elements with lazy data providers
    pub uniform_type_identifier: BorrowedStrPtr<'a>,
    pub content: BorrowedArray<'a, u8>,
}

#[repr(C)]
#[derive(Debug)]
pub struct PasteboardItem<'a> {
    elements: BorrowedArray<'a, CombinedItemElement<'a>>,
}

impl PasteboardItem<'_> {
    pub(crate) fn to_ns_pasteboard_item(&self) -> anyhow::Result<Retained<ProtocolObject<dyn NSPasteboardWriting>>> {
        let elements = self.elements.as_slice()?;
        let item = NSPasteboardItem::new();
        for element in elements {
            let uti = copy_to_ns_string(&element.uniform_type_identifier)?;
            let data = NSData::with_bytes(element.content.as_slice()?);
            assert!(item.setData_forType(&data, &uti));
        }
        Ok(ProtocolObject::from_retained(item))
    }

    fn copy_to_ns_array(items: &[Self]) -> anyhow::Result<Retained<NSArray<ProtocolObject<dyn NSPasteboardWriting>>>> {
        let array = NSMutableArray::<ProtocolObject<dyn NSPasteboardWriting>>::arrayWithCapacity(items.len());
        for item in items {
            let object: Retained<ProtocolObject<dyn NSPasteboardWriting>> = item.to_ns_pasteboard_item()?;
            array.addObject(&object);
        }
        Ok(array.into_super())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_write_objects(items: BorrowedArray<PasteboardItem>) -> bool {
    ffi_boundary("pasteboard_write_objects", || {
        with_pasteboard(&PasteboardType::Global, |pasteboard| {
            debug!("pasteboard_write_objects: {items:?}");
            let objects = PasteboardItem::copy_to_ns_array(items.as_slice()?)?;
            Ok(pasteboard.writeObjects(&objects))
        })
    })
}

#[repr(C)]
pub struct PasteboardContentResult {
    items: AutoDropArray<AutoDropArray<u8>>,
}

impl PanicDefault for PasteboardContentResult {
    fn default() -> Self {
        Self {
            items: AutoDropArray::new(Box::new([])),
        }
    }
}

fn pasteboard_type_by_str_ptr(pasteboard_name: &BorrowedStrPtr) -> PasteboardType {
    copy_to_ns_string_if_not_null(pasteboard_name).map_or(PasteboardType::Global, PasteboardType::WithName)
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_read_change_count() -> isize {
    ffi_boundary("pasteboard_read_change_count", || {
        let result = with_pasteboard(&PasteboardType::Global, |pasteboard| pasteboard.changeCount());
        Ok(result)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_read_items_count() -> isize {
    ffi_boundary("pasteboard_read_items_count", || {
        let result = with_pasteboard(&PasteboardType::Global, |pasteboard| {
            pasteboard.pasteboardItems().map_or(0, |items| items.count()) as isize
        });
        Ok(result)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_read_items_of_type(
    pasteboard_name: BorrowedStrPtr,
    uniform_type_identifier: BorrowedStrPtr,
) -> PasteboardContentResult {
    ffi_boundary("pasteboard_read_content_for_type", || {
        with_pasteboard(&pasteboard_type_by_str_ptr(&pasteboard_name), |pasteboard| {
            let uti = copy_to_ns_string(&uniform_type_identifier)?;
            let items = pasteboard.pasteboardItems().context("Can't retrieve items")?;
            let items: Box<[_]> = items
                .iter()
                .filter_map(|item| item.dataForType(&uti))
                .map(|data| AutoDropArray::new(data.to_vec().into_boxed_slice()))
                .collect();
            Ok(PasteboardContentResult {
                items: AutoDropArray::new(items),
            })
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_read_file_items(pasteboard_name: BorrowedStrPtr) -> PasteboardContentResult {
    ffi_boundary("pasteboard_read_file_items", || {
        with_pasteboard(&pasteboard_type_by_str_ptr(&pasteboard_name), |pasteboard| {
            let class_array = NSArray::from_slice(&[NSURL::class()]);

            let options = NSDictionary::from_slices(
                &[unsafe { NSPasteboardURLReadingFileURLsOnlyKey }],
                &[&*Retained::<AnyObject>::from(NSNumber::numberWithBool(true))],
            );
            let urls = unsafe { pasteboard.readObjectsForClasses_options(&class_array, Some(&*options)) }.context("No items")?;

            let urls: Box<_> = urls
                .iter()
                .map(|url| url.downcast::<NSURL>().expect("It must be NSURL"))
                .filter_map(|url| url_to_file_path_string(&url))
                .map(|url_ns_str| {
                    let c_str = unsafe { CStr::from_ptr(url_ns_str.UTF8String()) };
                    AutoDropArray::new(Box::<[u8]>::from(c_str.to_bytes()))
                })
                .collect();

            Ok(PasteboardContentResult {
                items: AutoDropArray::new(urls),
            })
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn pasteboard_content_drop(content: PasteboardContentResult) {
    ffi_boundary("pasteboard_content_drop", || {
        drop(content);
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use test_log::test;

    use log::info;
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{NSPasteboardItem, NSPasteboardTypeString, NSURLNSPasteboardSupport};
    use objc2_foundation::{NSArray, NSObjectNSComparisonMethods, NSString, NSURL, ns_string};

    use crate::macos::pasteboard::{PasteboardType, with_pasteboard};

    #[test]
    fn test_pasteboard_can_store_and_return_string() {
        with_pasteboard(&PasteboardType::WithUniqueName, |pasteboard| {
            let original_string = ns_string!("Hello😃World");
            unsafe { info!("NSPasteboardTypeString: {NSPasteboardTypeString:?}") };
            pasteboard.clearContents();
            pasteboard.setString_forType(original_string, unsafe { NSPasteboardTypeString });
            let types = pasteboard.types();
            assert!(types.is_some_and(|types| unsafe { types.doesContain(NSPasteboardTypeString) }));
            let string_from_pasteboard = pasteboard.stringForType(unsafe { NSPasteboardTypeString });
            assert_eq!(Some(original_string), string_from_pasteboard.as_deref());
        });
    }

    #[test]
    fn test_empty_pasteboard_doesnt_contain_string() {
        with_pasteboard(&PasteboardType::WithUniqueName, |pasteboard| {
            pasteboard.clearContents();
            let string_from_pasteboard = pasteboard.stringForType(unsafe { NSPasteboardTypeString });
            assert_eq!(None, string_from_pasteboard);
        });
    }

    #[test]
    fn test_store_custom_type() {
        with_pasteboard(&PasteboardType::WithUniqueName, |pasteboard| {
            let metadata_string = ns_string!("some metadata");
            let my_pasteboard_type = ns_string!("org.jetbrains.kdt.meta-string");
            pasteboard.clearContents();
            let new_types = NSArray::from_slice(&[my_pasteboard_type]);
            let change_count = unsafe { pasteboard.declareTypes_owner(&new_types, None) };
            // when we declare type it immediately shows in pasteboard types
            // after clearContents it's gone
            info!("types1: {:?}", pasteboard.types());
            info!("change: {change_count:?}");
            pasteboard.setString_forType(metadata_string, my_pasteboard_type);
            info!("types2: {:?}", pasteboard.types());
            let string_from_pasteboard = pasteboard.stringForType(unsafe { NSPasteboardTypeString });
            assert_eq!(None, string_from_pasteboard);
            let meta_string_from_pasteboard = pasteboard.stringForType(my_pasteboard_type);
            assert_eq!(Some(metadata_string), meta_string_from_pasteboard.as_deref());
            info!("pasteboard items: {:?}", pasteboard.pasteboardItems());
        });
    }

    #[test]
    fn test_store_custom_type_with_string() {
        with_pasteboard(&PasteboardType::WithUniqueName, |pasteboard| {
            let metadata_string = ns_string!("some metadata");
            let original_string = ns_string!("Hello");
            let my_pasteboard_type = ns_string!("org.jetbrains.kdt.meta-string");
            pasteboard.clearContents();
            let new_types = NSArray::from_slice(&[my_pasteboard_type]);
            let change_count = unsafe { pasteboard.declareTypes_owner(&new_types, None) };
            pasteboard.clearContents();
            info!("types1: {:?}", pasteboard.types());
            info!("change: {change_count:?}");
            pasteboard.setString_forType(metadata_string, my_pasteboard_type);
            pasteboard.setString_forType(original_string, unsafe { NSPasteboardTypeString });
            info!("types2: {:?}", pasteboard.types());
            let string_from_pasteboard = pasteboard.stringForType(unsafe { NSPasteboardTypeString });
            assert_eq!(Some(original_string), string_from_pasteboard.as_deref());
            let meta_string_from_pasteboard = pasteboard.stringForType(my_pasteboard_type);
            assert_eq!(Some(metadata_string), meta_string_from_pasteboard.as_deref());
        });
    }

    #[test]
    fn test_store_two_files() {
        with_pasteboard(&PasteboardType::WithUniqueName, |general| {
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
            info!("types: {types:?}");
            assert_eq!(Some(url1), NSURL::URLFromPasteboard(general));
            assert_eq!(Some(2), general.pasteboardItems().map(|items| items.count()));
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
        with_pasteboard(&PasteboardType::WithUniqueName, |general| {
            let original_string = ns_string!("Hello");
            general.clearContents();
            let url1 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("mouse.rs").to_str().unwrap()));
            let url2 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("string.rs").to_str().unwrap()));
            // url.writeToPasteboard(&general); // apparently doesn't work
            general.writeObjects(&NSArray::from_slice(&[
                ProtocolObject::from_ref(&*url1),
                ProtocolObject::from_ref(&*url2),
            ]));
            general.setString_forType(original_string, unsafe { NSPasteboardTypeString });
            let types = general.types();
            info!("types: {types:?}");
            assert_eq!(Some(url1), NSURL::URLFromPasteboard(general));
            let items = general.pasteboardItems().unwrap().to_vec();
            assert_eq!(2, items.len());
        });
    }

    #[test]
    fn test_create_pasteboard_item() {
        with_pasteboard(&PasteboardType::WithUniqueName, |general| {
            let item = NSPasteboardItem::new();
            let original_string = ns_string!("Hello");
            general.clearContents();
            let url1 = NSURL::fileURLWithPath(&NSString::from_str(get_source_file("mouse.rs").to_str().unwrap()));
            item.setString_forType(original_string, unsafe { NSPasteboardTypeString });
            let result = general.writeObjects(&NSArray::from_slice(&[
                ProtocolObject::from_ref(&*url1),
                ProtocolObject::from_ref(&*item),
            ]));
            assert!(result);
        });
    }

    #[test]
    fn test_clear_is_required_before_write_objects() {
        with_pasteboard(&PasteboardType::WithUniqueName, |general| {
            general.clearContents();
            let item1 = NSPasteboardItem::new();
            item1.setString_forType(ns_string!("Hello"), unsafe { NSPasteboardTypeString });
            let result1 = general.writeObjects(&NSArray::from_slice(&[ProtocolObject::from_ref(&*item1)]));
            assert!(result1);

            let item2 = NSPasteboardItem::new();
            item2.setString_forType(ns_string!("World"), unsafe { NSPasteboardTypeString });
            let result2 = general.writeObjects(&NSArray::from_slice(&[ProtocolObject::from_ref(&*item2)]));
            // this might be false if the application lost pasteboard ownership
            assert!(result2);
        });
    }

    #[test]
    fn test_can_create_url_from_path_with_spaces() {
        let url = NSString::from_str("https://www.jetbrains.com/idea/download/foo bar.txt");
        // Spaces will be replaced with %20, but apparently when it's runned from JVM it returns None
        let ns_url = NSURL::URLWithString(&url);
        assert!(ns_url.is_some());
    }
}
