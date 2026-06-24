use desktop_common::logger::ffi_boundary;

use windows::Win32::{
    Foundation::{S_FALSE, S_OK},
    System::{
        Com::IDataObject,
        DataExchange::GetClipboardSequenceNumber,
        Ole::{OleFlushClipboard, OleGetClipboard, OleSetClipboard},
    },
};
use windows_core::{Error as WinError, Interface};

use super::{
    clipboard::{ClipboardBoolResult, ClipboardDataObjectResult, ClipboardOperationResult, operation_result},
    com::ComInterfaceRawPtr,
    data_transfer::DataFormat,
};

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_sequence_number() -> u32 {
    ffi_boundary("clipboard_get_sequence_number", || unsafe { Ok(GetClipboardSequenceNumber()) })
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard_get_html_format_id() -> u32 {
    ffi_boundary("clipboard_get_html_format_id", || Ok(DataFormat::HtmlFragment.id()))
}

/// `OleGetClipboard`: returns the data object currently on the clipboard.
///
/// The caller owns the returned reference and must release it (`com_data_object_release`).
#[unsafe(no_mangle)]
pub extern "C" fn clipboard_read_result() -> ClipboardDataObjectResult {
    ffi_boundary("clipboard_read_result", || {
        Ok(ClipboardDataObjectResult::from_result(clipboard_read_impl()))
    })
}

fn clipboard_read_impl() -> anyhow::Result<ComInterfaceRawPtr> {
    // SAFETY: Clipboard FFI is called on the application's UI thread after it has
    // initialized OLE as an STA, which is required for OLE clipboard access.
    let data_object = unsafe { OleGetClipboard()? };
    Ok(ComInterfaceRawPtr::from_interface(&data_object)?)
}

/// `OleSetClipboard`: places `data_object_ptr` on the clipboard using delayed rendering.
///
/// The clipboard holds only a pointer to the data object until [`clipboard_flush_result`]
/// renders the data, the clipboard is cleared, or another object is set. OLE takes its own
/// reference, so the caller may release its handle independently after this returns.
#[unsafe(no_mangle)]
pub extern "C" fn clipboard_set_data_object_result(data_object_ptr: ComInterfaceRawPtr) -> ClipboardOperationResult {
    ffi_boundary("clipboard_set_data_object_result", || {
        Ok(operation_result(clipboard_set_data_object_impl(&data_object_ptr)))
    })
}

fn clipboard_set_data_object_impl(data_object_ptr: &ComInterfaceRawPtr) -> anyhow::Result<()> {
    let data_object = data_object_ptr.cast::<IDataObject>()?;
    // SAFETY: Clipboard FFI is called on the application's initialized OLE STA.
    // `data_object` is a live IDataObject reference reconstructed from the FFI handle
    // and remains valid for the duration of this call.
    unsafe { OleSetClipboard(&data_object)? };
    Ok(())
}

/// `OleFlushClipboard`: renders the set data object into the clipboard.
///
/// Renders the data of the data object previously published with [`clipboard_set_data_object_result`]
/// and releases OLE's reference to it, so the data survives after the application exits.
#[unsafe(no_mangle)]
pub extern "C" fn clipboard_flush_result() -> ClipboardOperationResult {
    ffi_boundary("clipboard_flush_result", || Ok(operation_result(clipboard_flush_impl())))
}

fn clipboard_flush_impl() -> anyhow::Result<()> {
    // SAFETY: Runs on the application's initialized OLE STA. OleFlushClipboard borrows no
    // caller-owned data.
    unsafe { OleFlushClipboard()? };
    Ok(())
}

/// `OleSetClipboard(NULL)`: empties the clipboard, releasing any data object previously set.
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

/// `OleIsCurrentClipboard`: reports whether `data_object_ptr` (a data object previously set
/// with [`clipboard_set_data_object_result`]) is still the one on the clipboard.
#[unsafe(no_mangle)]
pub extern "C" fn clipboard_is_current_data_object_result(data_object_ptr: ComInterfaceRawPtr) -> ClipboardBoolResult {
    ffi_boundary("clipboard_is_current_data_object_result", || {
        Ok(ClipboardBoolResult::from_result(clipboard_is_current_data_object_impl(
            &data_object_ptr,
        )))
    })
}

fn clipboard_is_current_data_object_impl(data_object_ptr: &ComInterfaceRawPtr) -> anyhow::Result<bool> {
    let data_object = data_object_ptr.cast::<IDataObject>()?;
    // The safe `OleIsCurrentClipboard` wrapper folds the meaningful S_OK/S_FALSE return into
    // `Result<()>`, so call the raw entry point to keep the distinction.
    windows_core::link!("ole32.dll" "system" fn OleIsCurrentClipboard(pdataobj: *mut core::ffi::c_void) -> windows_core::HRESULT);
    // SAFETY: Runs on the application's initialized OLE STA. `data_object` is a live
    // IDataObject reference; its raw pointer stays valid for the duration of the call.
    let result = unsafe { OleIsCurrentClipboard(data_object.as_raw()) };
    match result {
        S_OK => Ok(true),
        S_FALSE => Ok(false),
        error => Err(WinError::from(error).into()),
    }
}
