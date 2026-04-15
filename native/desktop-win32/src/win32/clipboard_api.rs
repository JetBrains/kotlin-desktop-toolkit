use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedArray, BorrowedStrPtr, FfiOption, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

use super::{
    clipboard::{Clipboard, ClipboardFormat},
    global_data::{hglobal_reader, hglobal_writer},
    strings::copy_from_utf8_string,
    window::Window,
    window_api::{WindowPtr, with_window},
};

type AutoDropByteArray = AutoDropArray<u8>;
type AutoDropUInt32Array = AutoDropArray<u32>;

trait IntoFfiOption<T> {
    fn into_ffi_option(self) -> anyhow::Result<FfiOption<T>>
    where
        T: desktop_common::logger::PanicDefault;
}

impl<T> IntoFfiOption<T> for anyhow::Result<T> {
    fn into_ffi_option(self) -> anyhow::Result<FfiOption<T>>
    where
        T: desktop_common::logger::PanicDefault,
    {
        match self {
            Ok(ok) => Ok(FfiOption::some(ok)),
            Err(err) => {
                log::trace!("failed to get data from Clipboard: {err}");
                Ok(FfiOption::none())
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_count_formats(owner: WindowPtr) -> i32 {
    with_window(&owner, "clipboard_count_formats", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let count = clipboard.count_available_formats()?;
        Ok(count)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_enum_formats(owner: WindowPtr) -> AutoDropUInt32Array {
    with_window(&owner, "clipboard_enum_formats", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let formats = clipboard.enum_available_formats()?;
        Ok(AutoDropArray::new(formats.into_boxed_slice()))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_is_format_available(owner: WindowPtr, data_format: u32) -> bool {
    with_window(&owner, "clipboard_is_format_available", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        clipboard.is_format_available(data_format)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_empty(owner: WindowPtr) {
    with_window(&owner, "clipboard_empty", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        clipboard.empty()
    });
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
pub extern "C" fn clipboard_try_get_data(owner: WindowPtr, data_format: u32) -> FfiOption<AutoDropByteArray> {
    with_window(&owner, "clipboard_try_get_data", |window| {
        clipboard_get_data_impl(window, data_format).into_ffi_option()
    })
}

fn clipboard_get_data_impl(owner: &Window, data_format: u32) -> anyhow::Result<AutoDropByteArray> {
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard
        .get_data(ClipboardFormat::Other(data_format))
        .and_then(|data| hglobal_reader::get_bytes(&data))
        .map(|bytes| AutoDropArray::new(bytes.into_boxed_slice()))
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_file_list(owner: WindowPtr) -> AutoDropArray<RustAllocatedStrPtr> {
    with_window(&owner, "clipboard_get_file_list", clipboard_get_file_list_impl)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_try_get_file_list(owner: WindowPtr) -> AutoDropArray<RustAllocatedStrPtr> {
    with_window(&owner, "clipboard_try_get_file_list", |window| {
        clipboard_get_file_list_impl(window).or_else(|err| {
            log::trace!("failed to get data from Clipboard: {err}");
            Ok(AutoDropArray::null())
        })
    })
}

fn clipboard_get_file_list_impl(owner: &Window) -> anyhow::Result<AutoDropArray<RustAllocatedStrPtr>> {
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard
        .get_data(ClipboardFormat::FileList)
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
pub extern "C" fn clipboard_try_get_html_fragment(owner: WindowPtr) -> FfiOption<RustAllocatedStrPtr> {
    with_window(&owner, "clipboard_try_get_html_fragment", |window| {
        clipboard_get_html_fragment_impl(window).into_ffi_option()
    })
}

fn clipboard_get_html_fragment_impl(owner: &Window) -> anyhow::Result<RustAllocatedStrPtr> {
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard
        .get_data(ClipboardFormat::HtmlFragment)
        .and_then(|data| hglobal_reader::get_html(&data))
        .map(RustAllocatedStrPtr::from_c_string)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_text(owner: WindowPtr) -> RustAllocatedStrPtr {
    with_window(&owner, "clipboard_get_text", clipboard_get_text_impl)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_try_get_text(owner: WindowPtr) -> FfiOption<RustAllocatedStrPtr> {
    with_window(&owner, "clipboard_try_get_text", |window| {
        clipboard_get_text_impl(window).into_ffi_option()
    })
}

fn clipboard_get_text_impl(owner: &Window) -> anyhow::Result<RustAllocatedStrPtr> {
    let clipboard = Clipboard::open_for_window(owner)?;
    clipboard
        .get_data(ClipboardFormat::Text)
        .and_then(|data| hglobal_reader::get_text(&data))
        .map(RustAllocatedStrPtr::from_c_string)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_data(owner: WindowPtr, data_format: u32, content: BorrowedArray<u8>) {
    with_window(&owner, "clipboard_set_data", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let mut data = hglobal_writer::new_bytes(content.as_slice()?)?;
        clipboard.set_data(ClipboardFormat::Other(data_format), &mut data)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_file_list(owner: WindowPtr, content: BorrowedArray<BorrowedStrPtr>) {
    with_window(&owner, "clipboard_set_data", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let files: anyhow::Result<Vec<&str>> = content.as_slice()?.iter().map(|str_ptr| str_ptr.as_str()).collect();
        let mut data = hglobal_writer::new_file_list(&files?)?;
        clipboard.set_data(ClipboardFormat::FileList, &mut data)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_html_fragment(owner: WindowPtr, content: BorrowedStrPtr) {
    with_window(&owner, "clipboard_set_html_fragment", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let fragment = copy_from_utf8_string(&content)?;
        let mut data = hglobal_writer::new_html(&fragment)?;
        clipboard.set_data(ClipboardFormat::HtmlFragment, &mut data)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_text(owner: WindowPtr, content: BorrowedStrPtr) {
    with_window(&owner, "clipboard_set_text", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let mut data = hglobal_writer::new_text(content.as_str()?)?;
        clipboard.set_data(ClipboardFormat::Text, &mut data)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_register_format(name: BorrowedStrPtr) -> u32 {
    ffi_boundary("clipboard_register_format", || {
        let format_name = copy_from_utf8_string(&name)?;
        Clipboard::register_format(&format_name)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_html_format_id() -> u32 {
    ffi_boundary("clipboard_get_html_format_id", || Ok(ClipboardFormat::HtmlFragment.id()))
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

#[unsafe(no_mangle)]
pub extern "C" fn native_u32_array_drop(array: AutoDropUInt32Array) {
    ffi_boundary("native_u32_array_drop", || {
        drop(array);
        Ok(())
    });
}
