use anyhow::Context;
use objc2::{MainThreadMarker, rc::Retained};
use objc2_app_kit::{NSModalResponseOK, NSOpenPanel, NSSavePanel};
use objc2_foundation::{NSString, NSURL};

use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

use crate::macos::string::copy_to_ns_string;

use super::{string::copy_to_c_string, url::url_to_file_path_string};

// see the dock: https://developer.apple.com/documentation/appkit/nssavepanel?language=objc
#[repr(C)]
pub struct CommonFileDialogParams<'a> {
    // Appearance
    pub title: BorrowedStrPtr<'a>,
    pub prompt: BorrowedStrPtr<'a>,
    pub message: BorrowedStrPtr<'a>,
    pub name_field_label: BorrowedStrPtr<'a>,
    pub name_field_string_value: BorrowedStrPtr<'a>,
    pub directory_url: BorrowedStrPtr<'a>,
    // Behaviour
    pub can_create_directories: bool,
    pub can_select_hidden_extension: bool,
    pub shows_hidden_files: bool,
    pub extensions_hidden: bool,
}

#[repr(C)]
pub struct OpenFileDialogParams {
    pub can_choose_files: bool,
    pub can_choose_directories: bool,
    pub resolves_aliases: bool,
    pub allows_multiple_selection: bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn open_file_dialog_run_modal(
    common_params: &CommonFileDialogParams,
    params: &OpenFileDialogParams,
) -> AutoDropArray<RustAllocatedStrPtr> {
    ffi_boundary("open_file_dialog_run_modal", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let result = open_file_dialog_run_modal_impl(mtm, common_params, params)?;
        Ok(result)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn save_file_dialog_run_modal(common_params: &CommonFileDialogParams) -> RustAllocatedStrPtr {
    ffi_boundary("save_file_dialog_run_modal", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let path = save_file_dialog_run_modal_impl(mtm, common_params)?;
        let result = if let Some(path_ns_str) = path {
            copy_to_c_string(&path_ns_str)?
        } else {
            RustAllocatedStrPtr::null()
        };
        Ok(result)
    })
}

fn apply_common_param(panel: &NSSavePanel, common_params: &CommonFileDialogParams) {
    unsafe { panel.setTitle(to_ns_string_if_not_null(&common_params.title).as_deref()) };
    unsafe { panel.setPrompt(to_ns_string_if_not_null(&common_params.prompt).as_deref()) }
    unsafe { panel.setMessage(to_ns_string_if_not_null(&common_params.message).as_deref()) }
    unsafe { panel.setNameFieldLabel(to_ns_string_if_not_null(&common_params.name_field_label).as_deref()) }
    if let Some(name_field_string_value) = to_ns_string_if_not_null(&common_params.name_field_string_value).as_deref() {
        unsafe { panel.setNameFieldStringValue(name_field_string_value) }
    }
    let directory_url_str = to_ns_string_if_not_null(&common_params.directory_url);
    let directory_url = directory_url_str.map(|directory_url_str| unsafe { NSURL::fileURLWithPath(&directory_url_str) });
    unsafe {
        panel.setDirectoryURL(directory_url.as_deref());
    }

    unsafe { panel.setCanCreateDirectories(common_params.can_create_directories) };
    unsafe { panel.setCanSelectHiddenExtension(common_params.can_select_hidden_extension) };
    unsafe {
        panel.setShowsHiddenFiles(common_params.shows_hidden_files);
    }
    unsafe { panel.setExtensionHidden(common_params.extensions_hidden) }
}

fn to_ns_string_if_not_null(str_ptr: &BorrowedStrPtr) -> Option<Retained<NSString>> {
    if str_ptr.is_not_null() {
        Some(copy_to_ns_string(str_ptr).unwrap())
    } else {
        None
    }
}

fn apply_open_file_dialog_params(panel: &NSOpenPanel, open_params: &OpenFileDialogParams) {
    unsafe {
        panel.setCanChooseFiles(open_params.can_choose_files);
        panel.setCanChooseDirectories(open_params.can_choose_directories);
        panel.setResolvesAliases(open_params.resolves_aliases);
        panel.setAllowsMultipleSelection(open_params.allows_multiple_selection);
    }
}

fn run_and_get_path(panel: &NSSavePanel) -> anyhow::Result<Option<Retained<NSString>>> {
    let success = unsafe { panel.runModal() == NSModalResponseOK };
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

fn open_file_dialog_run_modal_impl(
    mtm: MainThreadMarker,
    common_params: &CommonFileDialogParams,
    open_params: &OpenFileDialogParams,
) -> anyhow::Result<AutoDropArray<RustAllocatedStrPtr>> {
    let panel = unsafe { NSOpenPanel::new(mtm) };
    apply_common_param(&panel, common_params);
    apply_open_file_dialog_params(&panel, open_params);
    let success = unsafe { panel.runModal() == NSModalResponseOK };
    let urls = if success {
        let urls = unsafe { panel.URLs() };
        urls.iter()
            .map(|url| {
                let ns_string = url_to_file_path_string(&url).context("Not file URL")?;
                copy_to_c_string(&ns_string).context("Can't convert string")
            })
            .collect::<anyhow::Result<Box<[RustAllocatedStrPtr]>>>()?
    } else {
        Box::new([])
    };
    Ok(AutoDropArray::new(urls))
}

fn save_file_dialog_run_modal_impl(
    mtm: MainThreadMarker,
    common_params: &CommonFileDialogParams,
) -> anyhow::Result<Option<Retained<NSString>>> {
    let panel = unsafe { NSSavePanel::new(mtm) };
    apply_common_param(&panel, common_params);
    run_and_get_path(&panel)
}
