use std::collections::HashSet;

use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedArray, BorrowedStrPtr, FfiOption, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

use windows::Win32::System::{
    Com::IDataObject,
    Ole::{OleFlushClipboard, OleGetClipboard, OleSetClipboard},
};

use super::{
    clipboard::Clipboard,
    clipboard_result::{
        ClipboardBoolResult, ClipboardByteArrayResult, ClipboardDataObjectResult, ClipboardFailure, ClipboardIntResult,
        ClipboardOperationResult, ClipboardStringArrayResult, ClipboardStringResult, ClipboardUInt32ArrayResult, operation_result,
    },
    com::ComInterfaceRawPtr,
    data_object_api::{AutoDropByteArray, AutoDropUInt32Array, IntoFfiOption},
    data_transfer::DataFormat,
    global_data::{HGlobalData, hglobal_reader, hglobal_writer},
    strings::copy_from_utf8_string,
    window::Window,
    window_api::{WindowPtr, with_window},
};

fn with_window_raw<R>(window_ptr: &WindowPtr, f: impl FnOnce(&Window) -> anyhow::Result<R>) -> anyhow::Result<R> {
    let window = unsafe { window_ptr.borrow::<Window>() };
    f(window)
}

fn open_unchanged_clipboard(window: &Window, expected_sequence: u32) -> anyhow::Result<Clipboard> {
    let clipboard = Clipboard::open_for_window(window)?;
    clipboard.ensure_sequence_unchanged(expected_sequence)?;
    Ok(clipboard)
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardSetItemKind {
    Bytes,
    Text,
    HtmlFragment,
    FileList,
}

#[repr(C)]
#[derive(Debug)]
pub struct ClipboardSetItem<'a> {
    pub kind: ClipboardSetItemKind,
    pub data_format: u32,
    pub bytes: BorrowedArray<'a, u8>,
    pub text: BorrowedStrPtr<'a>,
    pub strings: BorrowedArray<'a, BorrowedStrPtr<'a>>,
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_count_formats(owner: WindowPtr) -> i32 {
    with_window(&owner, "clipboard_count_formats", clipboard_count_formats_impl)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_count_formats_result(owner: WindowPtr) -> ClipboardIntResult {
    ffi_boundary("clipboard_count_formats_result", || {
        Ok(ClipboardIntResult::from_result(with_window_raw(
            &owner,
            clipboard_count_formats_impl,
        )))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_count_formats_if_unchanged_result(owner: WindowPtr, expected_sequence: u32) -> ClipboardIntResult {
    ffi_boundary("clipboard_count_formats_if_unchanged_result", || {
        Ok(ClipboardIntResult::from_result(with_window_raw(&owner, |window| {
            clipboard_count_formats_if_unchanged_impl(window, expected_sequence)
        })))
    })
}

fn clipboard_count_formats_impl(window: &Window) -> anyhow::Result<i32> {
    let clipboard = Clipboard::open_for_window(window)?;
    let count = clipboard.count_available_formats()?;
    Ok(count)
}

fn clipboard_count_formats_if_unchanged_impl(window: &Window, expected_sequence: u32) -> anyhow::Result<i32> {
    let clipboard = open_unchanged_clipboard(window, expected_sequence)?;
    clipboard.count_available_formats()
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_enum_formats(owner: WindowPtr) -> AutoDropUInt32Array {
    with_window(&owner, "clipboard_enum_formats", clipboard_enum_formats_impl)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_enum_formats_result(owner: WindowPtr) -> ClipboardUInt32ArrayResult {
    ffi_boundary("clipboard_enum_formats_result", || {
        Ok(ClipboardUInt32ArrayResult::from_result(with_window_raw(
            &owner,
            clipboard_enum_formats_impl,
        )))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_enum_formats_if_unchanged_result(owner: WindowPtr, expected_sequence: u32) -> ClipboardUInt32ArrayResult {
    ffi_boundary("clipboard_enum_formats_if_unchanged_result", || {
        Ok(ClipboardUInt32ArrayResult::from_result(with_window_raw(&owner, |window| {
            clipboard_enum_formats_if_unchanged_impl(window, expected_sequence)
        })))
    })
}

fn clipboard_enum_formats_impl(window: &Window) -> anyhow::Result<AutoDropUInt32Array> {
    let clipboard = Clipboard::open_for_window(window)?;
    let formats = clipboard.enum_available_formats()?;
    Ok(AutoDropArray::new(formats.into_boxed_slice()))
}

fn clipboard_enum_formats_if_unchanged_impl(window: &Window, expected_sequence: u32) -> anyhow::Result<AutoDropUInt32Array> {
    let clipboard = open_unchanged_clipboard(window, expected_sequence)?;
    let formats = clipboard.enum_available_formats()?;
    Ok(AutoDropArray::new(formats.into_boxed_slice()))
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_is_format_available(owner: WindowPtr, data_format: u32) -> bool {
    with_window(&owner, "clipboard_is_format_available", |window| {
        clipboard_is_format_available_impl(window, data_format)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_is_format_available_result(owner: WindowPtr, data_format: u32) -> ClipboardBoolResult {
    ffi_boundary("clipboard_is_format_available_result", || {
        Ok(ClipboardBoolResult::from_result(with_window_raw(&owner, |window| {
            clipboard_is_format_available_impl(window, data_format)
        })))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_is_format_available_if_unchanged_result(
    owner: WindowPtr,
    data_format: u32,
    expected_sequence: u32,
) -> ClipboardBoolResult {
    ffi_boundary("clipboard_is_format_available_if_unchanged_result", || {
        Ok(ClipboardBoolResult::from_result(with_window_raw(&owner, |window| {
            clipboard_is_format_available_if_unchanged_impl(window, data_format, expected_sequence)
        })))
    })
}

fn clipboard_is_format_available_impl(window: &Window, data_format: u32) -> anyhow::Result<bool> {
    let clipboard = Clipboard::open_for_window(window)?;
    clipboard.is_format_available(data_format)
}

fn clipboard_is_format_available_if_unchanged_impl(window: &Window, data_format: u32, expected_sequence: u32) -> anyhow::Result<bool> {
    let clipboard = open_unchanged_clipboard(window, expected_sequence)?;
    clipboard.is_format_available(data_format)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_empty(owner: WindowPtr) {
    with_window(&owner, "clipboard_empty", clipboard_empty_impl);
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_empty_result(owner: WindowPtr) -> ClipboardOperationResult {
    ffi_boundary("clipboard_empty_result", || {
        Ok(operation_result(with_window_raw(&owner, clipboard_empty_impl)))
    })
}

fn clipboard_empty_impl(window: &Window) -> anyhow::Result<()> {
    let clipboard = Clipboard::open_for_window(window)?;
    clipboard.empty()
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_sequence_number() -> u32 {
    ffi_boundary("clipboard_get_sequence_number", || Ok(Clipboard::get_sequence_number()))
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_data(owner: WindowPtr, data_format: u32) -> AutoDropByteArray {
    with_window(&owner, "clipboard_get_data", |window| clipboard_get_data_impl(window, data_format))
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_data_result(owner: WindowPtr, data_format: u32) -> ClipboardByteArrayResult {
    ffi_boundary("clipboard_get_data_result", || {
        Ok(ClipboardByteArrayResult::from_result(with_window_raw(&owner, |window| {
            clipboard_get_data_impl(window, data_format)
        })))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_data_if_unchanged_result(
    owner: WindowPtr,
    data_format: u32,
    expected_sequence: u32,
) -> ClipboardByteArrayResult {
    ffi_boundary("clipboard_get_data_if_unchanged_result", || {
        Ok(ClipboardByteArrayResult::from_result(with_window_raw(&owner, |window| {
            clipboard_get_data_if_unchanged_impl(window, data_format, expected_sequence)
        })))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_try_get_data(owner: WindowPtr, data_format: u32) -> FfiOption<AutoDropByteArray> {
    with_window(&owner, "clipboard_try_get_data", |window| {
        clipboard_get_data_impl(window, data_format).into_ffi_option()
    })
}

fn clipboard_get_data_impl(owner: &Window, data_format: u32) -> anyhow::Result<AutoDropByteArray> {
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard_get_data_from_open_clipboard(&clipboard, data_format)
}

fn clipboard_get_data_if_unchanged_impl(owner: &Window, data_format: u32, expected_sequence: u32) -> anyhow::Result<AutoDropByteArray> {
    let clipboard = open_unchanged_clipboard(owner, expected_sequence)?;
    clipboard_get_data_from_open_clipboard(&clipboard, data_format)
}

fn clipboard_get_data_from_open_clipboard(clipboard: &Clipboard, data_format: u32) -> anyhow::Result<AutoDropByteArray> {
    clipboard
        .get_data(DataFormat::Other(data_format))
        .and_then(|data| hglobal_reader::get_bytes(&data))
        .map(|bytes| AutoDropArray::new(bytes.into_boxed_slice()))
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_file_list(owner: WindowPtr) -> AutoDropArray<RustAllocatedStrPtr> {
    with_window(&owner, "clipboard_get_file_list", clipboard_get_file_list_impl)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_file_list_result(owner: WindowPtr) -> ClipboardStringArrayResult {
    ffi_boundary("clipboard_get_file_list_result", || {
        Ok(ClipboardStringArrayResult::from_result(with_window_raw(
            &owner,
            clipboard_get_file_list_impl,
        )))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_file_list_if_unchanged_result(owner: WindowPtr, expected_sequence: u32) -> ClipboardStringArrayResult {
    ffi_boundary("clipboard_get_file_list_if_unchanged_result", || {
        Ok(ClipboardStringArrayResult::from_result(with_window_raw(&owner, |window| {
            clipboard_get_file_list_if_unchanged_impl(window, expected_sequence)
        })))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_try_get_file_list(owner: WindowPtr) -> FfiOption<AutoDropArray<RustAllocatedStrPtr>> {
    with_window(&owner, "clipboard_try_get_file_list", |window| {
        clipboard_get_file_list_impl(window).into_ffi_option()
    })
}

fn clipboard_get_file_list_impl(owner: &Window) -> anyhow::Result<AutoDropArray<RustAllocatedStrPtr>> {
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard_get_file_list_from_open_clipboard(&clipboard)
}

fn clipboard_get_file_list_if_unchanged_impl(owner: &Window, expected_sequence: u32) -> anyhow::Result<AutoDropArray<RustAllocatedStrPtr>> {
    let clipboard = open_unchanged_clipboard(owner, expected_sequence)?;
    clipboard_get_file_list_from_open_clipboard(&clipboard)
}

fn clipboard_get_file_list_from_open_clipboard(clipboard: &Clipboard) -> anyhow::Result<AutoDropArray<RustAllocatedStrPtr>> {
    clipboard
        .get_data(DataFormat::FileList)
        .and_then(|data| hglobal_reader::get_file_list(&data))
        .map(|file_list| file_list.into_iter().map(RustAllocatedStrPtr::from_c_string).collect())
        .map(AutoDropArray::new)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_html_fragment(owner: WindowPtr) -> RustAllocatedStrPtr {
    with_window(&owner, "clipboard_get_html_fragment", |window| {
        clipboard_get_html_fragment_impl(window)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_html_fragment_result(owner: WindowPtr) -> ClipboardStringResult {
    ffi_boundary("clipboard_get_html_fragment_result", || {
        Ok(ClipboardStringResult::from_result(with_window_raw(
            &owner,
            clipboard_get_html_fragment_impl,
        )))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_html_fragment_if_unchanged_result(owner: WindowPtr, expected_sequence: u32) -> ClipboardStringResult {
    ffi_boundary("clipboard_get_html_fragment_if_unchanged_result", || {
        Ok(ClipboardStringResult::from_result(with_window_raw(&owner, |window| {
            clipboard_get_html_fragment_if_unchanged_impl(window, expected_sequence)
        })))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_try_get_html_fragment(owner: WindowPtr) -> FfiOption<RustAllocatedStrPtr> {
    with_window(&owner, "clipboard_try_get_html_fragment", |window| {
        clipboard_get_html_fragment_impl(window).into_ffi_option()
    })
}

fn clipboard_get_html_fragment_impl(owner: &Window) -> anyhow::Result<RustAllocatedStrPtr> {
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard_get_html_fragment_from_open_clipboard(&clipboard)
}

fn clipboard_get_html_fragment_if_unchanged_impl(owner: &Window, expected_sequence: u32) -> anyhow::Result<RustAllocatedStrPtr> {
    let clipboard = open_unchanged_clipboard(owner, expected_sequence)?;
    clipboard_get_html_fragment_from_open_clipboard(&clipboard)
}

fn clipboard_get_html_fragment_from_open_clipboard(clipboard: &Clipboard) -> anyhow::Result<RustAllocatedStrPtr> {
    clipboard
        .get_data(DataFormat::HtmlFragment)
        .and_then(|data| hglobal_reader::get_html(&data))
        .map(RustAllocatedStrPtr::from_c_string)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_text(owner: WindowPtr) -> RustAllocatedStrPtr {
    with_window(&owner, "clipboard_get_text", clipboard_get_text_impl)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_text_result(owner: WindowPtr) -> ClipboardStringResult {
    ffi_boundary("clipboard_get_text_result", || {
        Ok(ClipboardStringResult::from_result(with_window_raw(&owner, clipboard_get_text_impl)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_text_if_unchanged_result(owner: WindowPtr, expected_sequence: u32) -> ClipboardStringResult {
    ffi_boundary("clipboard_get_text_if_unchanged_result", || {
        Ok(ClipboardStringResult::from_result(with_window_raw(&owner, |window| {
            clipboard_get_text_if_unchanged_impl(window, expected_sequence)
        })))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_try_get_text(owner: WindowPtr) -> FfiOption<RustAllocatedStrPtr> {
    with_window(&owner, "clipboard_try_get_text", |window| {
        clipboard_get_text_impl(window).into_ffi_option()
    })
}

fn clipboard_get_text_impl(owner: &Window) -> anyhow::Result<RustAllocatedStrPtr> {
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard_get_text_from_open_clipboard(&clipboard)
}

fn clipboard_get_text_if_unchanged_impl(owner: &Window, expected_sequence: u32) -> anyhow::Result<RustAllocatedStrPtr> {
    let clipboard = open_unchanged_clipboard(owner, expected_sequence)?;
    clipboard_get_text_from_open_clipboard(&clipboard)
}

fn clipboard_get_text_from_open_clipboard(clipboard: &Clipboard) -> anyhow::Result<RustAllocatedStrPtr> {
    clipboard
        .get_data(DataFormat::Text)
        .and_then(|data| hglobal_reader::get_text(&data))
        .map(RustAllocatedStrPtr::from_c_string)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_data(owner: WindowPtr, data_format: u32, content: BorrowedArray<u8>) {
    with_window(&owner, "clipboard_set_data", |window| {
        clipboard_set_data_impl(window, data_format, &content)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_data_result(owner: WindowPtr, data_format: u32, content: BorrowedArray<u8>) -> ClipboardOperationResult {
    ffi_boundary("clipboard_set_data_result", || {
        Ok(operation_result(with_window_raw(&owner, |window| {
            clipboard_set_data_impl(window, data_format, &content)
        })))
    })
}

fn clipboard_set_data_impl(owner: &Window, data_format: u32, content: &BorrowedArray<u8>) -> anyhow::Result<()> {
    let data = hglobal_writer::new_bytes(content.as_slice()?)?;
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard.set_data_items(&mut [(DataFormat::Other(data_format), data)])
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_file_list(owner: WindowPtr, content: BorrowedArray<BorrowedStrPtr>) {
    with_window(&owner, "clipboard_set_file_list", |window| {
        clipboard_set_file_list_impl(window, &content)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_file_list_result(owner: WindowPtr, content: BorrowedArray<BorrowedStrPtr>) -> ClipboardOperationResult {
    ffi_boundary("clipboard_set_file_list_result", || {
        Ok(operation_result(with_window_raw(&owner, |window| {
            clipboard_set_file_list_impl(window, &content)
        })))
    })
}

fn clipboard_set_file_list_impl(owner: &Window, content: &BorrowedArray<BorrowedStrPtr>) -> anyhow::Result<()> {
    let files: anyhow::Result<Vec<&str>> = content.as_slice()?.iter().map(|str_ptr| str_ptr.as_str()).collect();
    let data = hglobal_writer::new_file_list(&files?)?;
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard.set_data_items(&mut [(DataFormat::FileList, data)])
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_html_fragment(owner: WindowPtr, content: BorrowedStrPtr) {
    with_window(&owner, "clipboard_set_html_fragment", |window| {
        clipboard_set_html_fragment_impl(window, &content)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_html_fragment_result(owner: WindowPtr, content: BorrowedStrPtr) -> ClipboardOperationResult {
    ffi_boundary("clipboard_set_html_fragment_result", || {
        Ok(operation_result(with_window_raw(&owner, |window| {
            clipboard_set_html_fragment_impl(window, &content)
        })))
    })
}

fn clipboard_set_html_fragment_impl(owner: &Window, content: &BorrowedStrPtr) -> anyhow::Result<()> {
    let fragment = copy_from_utf8_string(content)?;
    let data = hglobal_writer::new_html(&fragment)?;
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard.set_data_items(&mut [(DataFormat::HtmlFragment, data)])
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_text(owner: WindowPtr, content: BorrowedStrPtr) {
    with_window(&owner, "clipboard_set_text", |window| clipboard_set_text_impl(window, &content));
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_text_result(owner: WindowPtr, content: BorrowedStrPtr) -> ClipboardOperationResult {
    ffi_boundary("clipboard_set_text_result", || {
        Ok(operation_result(with_window_raw(&owner, |window| {
            clipboard_set_text_impl(window, &content)
        })))
    })
}

fn clipboard_set_text_impl(owner: &Window, content: &BorrowedStrPtr) -> anyhow::Result<()> {
    let data = hglobal_writer::new_text(content.as_str()?)?;
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard.set_data_items(&mut [(DataFormat::Text, data)])
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_items_result(owner: WindowPtr, content: BorrowedArray<ClipboardSetItem>) -> ClipboardOperationResult {
    ffi_boundary("clipboard_set_items_result", || {
        Ok(operation_result(with_window_raw(&owner, |window| {
            clipboard_set_items_impl(window, &content)
        })))
    })
}

fn clipboard_set_items_impl(owner: &Window, content: &BorrowedArray<ClipboardSetItem>) -> anyhow::Result<()> {
    let mut entries = clipboard_set_items_to_data(content.as_slice()?)?;
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard.set_data_items(&mut entries)
}

fn clipboard_set_items_to_data(items: &[ClipboardSetItem]) -> anyhow::Result<Vec<(DataFormat, HGlobalData)>> {
    let mut seen_formats = HashSet::with_capacity(items.len());
    let mut entries = Vec::with_capacity(items.len());

    for item in items {
        let (format, data) = clipboard_set_item_to_data(item)?;
        if !seen_formats.insert(format.id()) {
            return Err(ClipboardFailure::invalid_data(format!("duplicate clipboard format {}", format.id())).into());
        }
        entries.push((format, data));
    }
    Ok(entries)
}

fn clipboard_set_item_to_data(item: &ClipboardSetItem) -> anyhow::Result<(DataFormat, HGlobalData)> {
    match item.kind {
        ClipboardSetItemKind::Bytes => Ok((
            DataFormat::Other(item.data_format),
            hglobal_writer::new_bytes(item.bytes.as_slice()?)?,
        )),
        ClipboardSetItemKind::Text => Ok((DataFormat::Text, hglobal_writer::new_text(item.text.as_str()?)?)),
        ClipboardSetItemKind::HtmlFragment => {
            let fragment = copy_from_utf8_string(&item.text)?;
            Ok((DataFormat::HtmlFragment, hglobal_writer::new_html(&fragment)?))
        }
        ClipboardSetItemKind::FileList => {
            let files: anyhow::Result<Vec<&str>> = item.strings.as_slice()?.iter().map(|str_ptr| str_ptr.as_str()).collect();
            Ok((DataFormat::FileList, hglobal_writer::new_file_list(&files?)?))
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_html_format_id() -> u32 {
    ffi_boundary("clipboard_get_html_format_id", || Ok(DataFormat::HtmlFragment.id()))
}

#[unsafe(no_mangle)]
pub extern "C" fn ole_clipboard_empty() {
    ffi_boundary("ole_clipboard_empty", ole_clipboard_empty_impl);
}

#[unsafe(no_mangle)]
pub extern "C" fn ole_clipboard_empty_result() -> ClipboardOperationResult {
    ffi_boundary("ole_clipboard_empty_result", || Ok(operation_result(ole_clipboard_empty_impl())))
}

fn ole_clipboard_empty_impl() -> anyhow::Result<()> {
    unsafe { OleSetClipboard(None)? };
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn ole_clipboard_get_data() -> ComInterfaceRawPtr {
    ffi_boundary("ole_clipboard_get_data", ole_clipboard_get_data_impl)
}

#[unsafe(no_mangle)]
pub extern "C" fn ole_clipboard_get_data_result() -> ClipboardDataObjectResult {
    ffi_boundary("ole_clipboard_get_data_result", || {
        Ok(ClipboardDataObjectResult::from_result(ole_clipboard_get_data_impl()))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ole_clipboard_get_data_if_unchanged_result(expected_sequence: u32) -> ClipboardDataObjectResult {
    ffi_boundary("ole_clipboard_get_data_if_unchanged_result", || {
        Ok(ClipboardDataObjectResult::from_result(ole_clipboard_get_data_if_unchanged_impl(
            expected_sequence,
        )))
    })
}

fn ole_clipboard_get_data_impl() -> anyhow::Result<ComInterfaceRawPtr> {
    let data_object = unsafe { OleGetClipboard()? };
    Ok(ComInterfaceRawPtr::from_interface(&data_object)?)
}

fn ole_clipboard_get_data_if_unchanged_impl(expected_sequence: u32) -> anyhow::Result<ComInterfaceRawPtr> {
    Clipboard::ensure_sequence_number_unchanged(expected_sequence)?;
    let data_object = unsafe { OleGetClipboard()? };
    Clipboard::ensure_sequence_number_unchanged(expected_sequence)?;
    Ok(ComInterfaceRawPtr::from_interface(&data_object)?)
}

#[unsafe(no_mangle)]
pub extern "C" fn ole_clipboard_set_data(data_object_ptr: ComInterfaceRawPtr) {
    ffi_boundary("ole_clipboard_set_data", || ole_clipboard_set_data_impl(&data_object_ptr));
}

#[unsafe(no_mangle)]
pub extern "C" fn ole_clipboard_set_data_result(data_object_ptr: ComInterfaceRawPtr) -> ClipboardOperationResult {
    ffi_boundary("ole_clipboard_set_data_result", || {
        Ok(operation_result(ole_clipboard_set_data_impl(&data_object_ptr)))
    })
}

fn ole_clipboard_set_data_impl(data_object_ptr: &ComInterfaceRawPtr) -> anyhow::Result<()> {
    let data_object = data_object_ptr.cast::<IDataObject>()?;
    unsafe { OleSetClipboard(&data_object)? };
    unsafe { OleFlushClipboard()? };
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn native_byte_array_drop(array: AutoDropByteArray) {
    ffi_boundary("native_byte_array_drop", || {
        drop(array);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn native_optional_byte_array_drop(optional: FfiOption<AutoDropByteArray>) {
    ffi_boundary("native_optional_byte_array_drop", || {
        drop(optional);
        Ok(())
    });
}
