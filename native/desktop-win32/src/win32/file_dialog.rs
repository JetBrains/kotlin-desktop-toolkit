use std::ffi::CString;

use desktop_common::ffi_utils::BorrowedStrPtr;

use windows::{
    Win32::{
        Foundation::ERROR_CANCELLED,
        System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
        UI::Shell::{
            FOS_ALLOWMULTISELECT, FOS_FORCESHOWHIDDEN, FOS_PICKFOLDERS, IFileDialog, IFileOpenDialog, IFileSaveDialog, IShellItem,
            SHCreateItemFromParsingName, SIGDN_FILESYSPATH,
        },
    },
    core::Result as WinResult,
};

use super::{
    strings::{copy_from_utf8_string, copy_from_wide_string},
    window::Window,
};

#[repr(C)]
pub struct FileDialogOptions<'a> {
    // Appearance
    pub title: BorrowedStrPtr<'a>,
    pub prompt: BorrowedStrPtr<'a>,
    pub name_field_label: BorrowedStrPtr<'a>,
    pub name_field_string_value: BorrowedStrPtr<'a>,
    pub directory_path: BorrowedStrPtr<'a>,
    // Behaviour
    pub shows_hidden_files: bool,
}

#[repr(C)]
pub struct FileOpenDialogOptions {
    pub choose_directories: bool,
    pub allows_multiple_selection: bool,
}

pub(crate) struct FileOpenDialog {
    file_dialog: IFileOpenDialog,
    is_multiple_choice: bool,
}

pub(crate) struct FileSaveDialog {
    file_dialog: IFileSaveDialog,
}

impl FileOpenDialog {
    pub(crate) fn new(common_dialog_options: &FileDialogOptions, open_dialog_options: &FileOpenDialogOptions) -> WinResult<Self> {
        let file_dialog: IFileOpenDialog =
            unsafe { CoCreateInstance(&windows::Win32::UI::Shell::FileOpenDialog, None, CLSCTX_INPROC_SERVER)? };
        apply_common_options(&file_dialog, common_dialog_options)?;
        let mut options = unsafe { file_dialog.GetOptions()? };
        if open_dialog_options.choose_directories {
            options |= FOS_PICKFOLDERS;
        }
        let is_multiple_choice = open_dialog_options.allows_multiple_selection;
        if is_multiple_choice {
            options |= FOS_ALLOWMULTISELECT;
        }
        unsafe { file_dialog.SetOptions(options)? };
        Ok(Self {
            file_dialog,
            is_multiple_choice,
        })
    }

    pub(crate) fn show(&self, owner: &Window) -> WinResult<Vec<CString>> {
        if let Err(err) = unsafe { self.file_dialog.Show(Some(owner.hwnd())) }
            && err.code() == ERROR_CANCELLED.into()
        {
            log::debug!("The user has cancelled the file open dialog");
            return Ok(vec![]);
        }
        if self.is_multiple_choice {
            retrieve_selected_items(&self.file_dialog)
        } else {
            let shell_item = unsafe { self.file_dialog.GetResult()? };
            parse_shell_item(&shell_item).map(|item| vec![item])
        }
    }
}

impl FileSaveDialog {
    pub(crate) fn new(common_dialog_options: &FileDialogOptions) -> WinResult<Self> {
        let file_dialog: IFileSaveDialog =
            unsafe { CoCreateInstance(&windows::Win32::UI::Shell::FileSaveDialog, None, CLSCTX_INPROC_SERVER)? };
        apply_common_options(&file_dialog, common_dialog_options)?;
        Ok(Self { file_dialog })
    }

    pub(crate) fn show(&self, owner: &Window) -> WinResult<CString> {
        if let Err(err) = unsafe { self.file_dialog.Show(Some(owner.hwnd())) }
            && err.code() == ERROR_CANCELLED.into()
        {
            log::debug!("The user has cancelled the file save dialog");
            return Ok(CString::default());
        }
        let shell_item = unsafe { self.file_dialog.GetResult()? };
        parse_shell_item(&shell_item)
    }
}

fn apply_common_options(file_dialog: &IFileDialog, common_dialog_options: &FileDialogOptions) -> WinResult<()> {
    if let Ok(title) = copy_from_utf8_string(&common_dialog_options.title) {
        unsafe { file_dialog.SetTitle(&title)? };
    }
    if let Ok(prompt) = copy_from_utf8_string(&common_dialog_options.prompt) {
        unsafe { file_dialog.SetOkButtonLabel(&prompt)? };
    }
    if let Ok(name_field_label) = copy_from_utf8_string(&common_dialog_options.name_field_label) {
        unsafe { file_dialog.SetFileNameLabel(&name_field_label)? };
    }
    if let Ok(name_field_string_value) = copy_from_utf8_string(&common_dialog_options.name_field_string_value) {
        unsafe { file_dialog.SetFileName(&name_field_string_value)? };
    }
    if let Ok(directory_path) = copy_from_utf8_string(&common_dialog_options.directory_path) {
        let directory_item: IShellItem = unsafe { SHCreateItemFromParsingName(&directory_path, None)? };
        unsafe { file_dialog.SetDefaultFolder(&directory_item)? };
    }
    if common_dialog_options.shows_hidden_files {
        let options = unsafe { file_dialog.GetOptions()? };
        unsafe { file_dialog.SetOptions(options | FOS_FORCESHOWHIDDEN)? };
    }
    Ok(())
}

fn retrieve_selected_items(file_open_dialog: &IFileOpenDialog) -> WinResult<Vec<CString>> {
    let selected_items = unsafe { file_open_dialog.GetResults()? };
    let items_count = unsafe { selected_items.GetCount()? };
    let enum_items = unsafe { selected_items.EnumItems()? };
    let mut shell_items = vec![None; items_count.try_into()?];
    let mut retrieved_items = 0;
    unsafe { enum_items.Next(shell_items.as_mut_slice(), Some(&raw mut retrieved_items))? };
    let shell_items = shell_items.into_iter().filter_map(|item| parse_shell_item(&item?).ok()).collect();
    Ok(shell_items)
}

fn parse_shell_item(shell_item: &IShellItem) -> WinResult<CString> {
    let shell_item_file_path = unsafe { shell_item.GetDisplayName(SIGDN_FILESYSPATH) }
        .inspect_err(|err| log::error!("failed to retrieve IShellItem's file system path: {err}"))?;
    copy_from_wide_string(unsafe { shell_item_file_path.as_wide() })
        .inspect_err(|err| log::error!("failed to convert IShellItem's file system path to UTF-8: {err}"))
}
