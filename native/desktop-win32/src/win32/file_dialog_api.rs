use std::ffi::NulError;

use desktop_common::ffi_utils::{AutoDropArray, RustAllocatedStrPtr};

use super::{
    file_dialog::{FileDialogOptions, FileOpenDialog, FileOpenDialogOptions, FileSaveDialog},
    window_api::{WindowPtr, with_window},
};

#[unsafe(no_mangle)]
pub extern "C" fn open_file_dialog_run_modal(
    window_ptr: WindowPtr,
    common_options: &FileDialogOptions,
    open_options: &FileOpenDialogOptions,
) -> AutoDropArray<RustAllocatedStrPtr> {
    with_window(&window_ptr, "open_file_dialog_run_modal", |window| {
        let file_dialog = FileOpenDialog::new(common_options, open_options)?;
        let results = file_dialog
            .show(window)?
            .into_iter()
            .map(RustAllocatedStrPtr::allocate)
            .collect::<Result<Box<[RustAllocatedStrPtr]>, NulError>>()?;
        Ok(AutoDropArray::new(results))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_file_dialog_run_modal(window_ptr: WindowPtr, common_options: &FileDialogOptions) -> RustAllocatedStrPtr {
    with_window(&window_ptr, "save_file_dialog_run_modal", |window| {
        let file_save_dialog = FileSaveDialog::new(common_options)?;
        let result = file_save_dialog.show(window)?;
        Ok(RustAllocatedStrPtr::from_c_string(result))
    })
}
