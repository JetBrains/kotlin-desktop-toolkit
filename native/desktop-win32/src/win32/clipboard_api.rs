use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedArray, BorrowedStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

use super::{
    clipboard::{Clipboard, ClipboardData, ClipboardFormat},
    strings::copy_from_utf8_string,
    window_api::{WindowPtr, with_window},
};

type AutoDropByteArray = AutoDropArray<u8>;
type AutoDropUInt32Array = AutoDropArray<u32>;

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_data(owner: WindowPtr, data_format: u32) -> AutoDropByteArray {
    with_window(&owner, "clipboard_get_text", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let data = clipboard.get_data(ClipboardFormat::Other(data_format))?;
        let content = data.get_bytes()?;
        Ok(AutoDropArray::new(content.into_boxed_slice()))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_text(owner: WindowPtr) -> RustAllocatedStrPtr {
    with_window(&owner, "clipboard_get_text", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let data = clipboard.get_data(ClipboardFormat::Text)?;
        let text = data.get_text()?;
        Ok(RustAllocatedStrPtr::from_c_string(text))
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
pub extern "C" fn clipboard_set_data(owner: WindowPtr, data_format: u32, content: BorrowedArray<u8>) {
    with_window(&owner, "clipboard_set_data", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let data = ClipboardData::new_bytes(content.as_slice()?, data_format)?;
        clipboard.set_data(&data)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_text(owner: WindowPtr, content: BorrowedStrPtr) {
    with_window(&owner, "clipboard_set_text", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let clipboard_data = ClipboardData::new_text(content.as_str()?)?;
        clipboard.set_data(&clipboard_data)
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
pub extern "C" fn clipboard_list_formats(owner: WindowPtr) -> AutoDropUInt32Array {
    with_window(&owner, "clipboard_list_formats", |window| {
        let clipboard = Clipboard::open_for_window(window)?;
        let formats = clipboard.list_available_formats()?;
        Ok(AutoDropArray::new(formats.into_boxed_slice()))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn native_byte_array_drop(array: AutoDropByteArray) {
    ffi_boundary("native_byte_array_drop", || {
        drop(array);
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
