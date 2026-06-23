use desktop_common::logger::ffi_boundary;

use windows::Win32::System::{
    Com::IDataObject,
    DataExchange::GetClipboardSequenceNumber,
    Ole::{OleFlushClipboard, OleGetClipboard, OleSetClipboard},
};

use super::{
    clipboard_result::{ClipboardDataObjectResult, ClipboardFailure, ClipboardOperationResult, operation_result},
    com::ComInterfaceRawPtr,
    data_transfer::DataFormat,
};

fn ensure_sequence_number_unchanged(expected_sequence: u32) -> anyhow::Result<()> {
    let actual_sequence = clipboard_get_sequence_number();
    if actual_sequence != expected_sequence {
        return Err(ClipboardFailure::changed(actual_sequence).into());
    }
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_sequence_number() -> u32 {
    ffi_boundary("clipboard_get_sequence_number", || unsafe { Ok(GetClipboardSequenceNumber()) })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_html_format_id() -> u32 {
    ffi_boundary("clipboard_get_html_format_id", || Ok(DataFormat::HtmlFragment.id()))
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_clear_result() -> ClipboardOperationResult {
    ffi_boundary("clipboard_clear_result", || Ok(operation_result(clipboard_clear_impl())))
}

fn clipboard_clear_impl() -> anyhow::Result<()> {
    // SAFETY: Clipboard FFI is called on the application's UI thread after it has
    // initialized OLE as an STA. Passing no data object is the OLE clipboard clear path.
    unsafe { OleSetClipboard(None)? };
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_read_if_unchanged_result(expected_sequence: u32) -> ClipboardDataObjectResult {
    ffi_boundary("clipboard_read_if_unchanged_result", || {
        Ok(ClipboardDataObjectResult::from_result(clipboard_read_if_unchanged_impl(
            expected_sequence,
        )))
    })
}

fn clipboard_read_if_unchanged_impl(expected_sequence: u32) -> anyhow::Result<ComInterfaceRawPtr> {
    ensure_sequence_number_unchanged(expected_sequence)?;
    // SAFETY: Clipboard FFI is called on the application's UI thread after it has
    // initialized OLE as an STA, which is required for OLE clipboard access.
    let data_object = unsafe { OleGetClipboard()? };
    ensure_sequence_number_unchanged(expected_sequence)?;
    Ok(ComInterfaceRawPtr::from_interface(&data_object)?)
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_write_data_object_result(data_object_ptr: ComInterfaceRawPtr) -> ClipboardOperationResult {
    ffi_boundary("clipboard_write_data_object_result", || {
        Ok(operation_result(clipboard_write_data_object_impl(&data_object_ptr)))
    })
}

fn clipboard_write_data_object_impl(data_object_ptr: &ComInterfaceRawPtr) -> anyhow::Result<()> {
    let data_object = data_object_ptr.cast::<IDataObject>()?;
    // SAFETY: Clipboard FFI is called on the application's initialized OLE STA.
    // `data_object` is a live IDataObject reference reconstructed from the FFI handle
    // and remains valid for the duration of this call.
    unsafe { OleSetClipboard(&data_object)? };
    // SAFETY: This runs on the same initialized OLE STA after OleSetClipboard succeeds.
    // OleFlushClipboard does not borrow caller-owned data.
    unsafe { OleFlushClipboard()? };
    Ok(())
}
