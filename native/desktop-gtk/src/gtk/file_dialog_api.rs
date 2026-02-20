use desktop_common::ffi_utils::BorrowedStrPtr;

#[repr(C)]
pub struct CommonFileDialogParams<'a> {
    pub modal: bool,
    pub title: BorrowedStrPtr<'a>,
    pub accept_label: BorrowedStrPtr<'a>,
    pub current_folder: BorrowedStrPtr<'a>,
}

#[repr(C)]
pub struct OpenFileDialogParams {
    pub select_directories: bool,
    pub allows_multiple_selection: bool,
}

#[repr(C)]
pub struct SaveFileDialogParams<'a> {
    pub name_field_string_value: BorrowedStrPtr<'a>,
}
