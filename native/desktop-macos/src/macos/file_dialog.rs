use anyhow::Context;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSModalResponse, NSOpenPanel};
use objc2_foundation::NSString;

use crate::{common::RustAllocatedStrPtr, logger::ffi_boundary};

use super::string::copy_to_c_string;

pub type FileDialogCallback = extern "C" fn(*const std::ffi::c_char);

#[repr(C)]
pub struct FileDialogParams {
    pub allow_file: bool,
    pub allow_folder: bool,
    pub allow_multiple_selection: bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn file_dialog_run_modal(params: FileDialogParams) -> RustAllocatedStrPtr {
    ffi_boundary("file_dialog_run_modal", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let path = file_dialog_run_model_impl(mtm, &params)?;
        let result = if let Some(path_ns_str) = path {
            copy_to_c_string(&path_ns_str)?
        } else {
            RustAllocatedStrPtr::null()
        };
        Ok(result)
    })
}

fn file_dialog_run_model_impl(mtm: MainThreadMarker, params: &FileDialogParams) -> anyhow::Result<Option<Retained<NSString>>> {
    const NSMODAL_RESPONSE_OK: NSModalResponse = 1;
    let panel = unsafe { NSOpenPanel::new(mtm) };
    let success = unsafe {
        panel.setCanChooseFiles(params.allow_file);
        panel.setCanChooseDirectories(params.allow_folder);
        panel.setAllowsMultipleSelection(params.allow_multiple_selection);
        panel.runModal() == NSMODAL_RESPONSE_OK
    };
    let result = if success {
        unsafe {
            let url = panel.URL().context("Expected URL")?;
            let path = url.path().context("Expected path")?;
            Some(path)
        }
    } else {
        None
    };
    Ok(result)
}
