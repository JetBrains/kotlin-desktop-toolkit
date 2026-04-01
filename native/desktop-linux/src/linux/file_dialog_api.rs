use desktop_common::ffi_utils::BorrowedArray;

#[repr(C)]
pub struct CommonFileDialogParams<'a> {
    pub modal: bool,
    pub title: BorrowedArray<'a, u8>,
    pub accept_label: BorrowedArray<'a, u8>,
    pub current_folder: BorrowedArray<'a, u8>,
}

#[repr(C)]
pub struct OpenFileDialogParams {
    pub select_directories: bool,
    pub allows_multiple_selection: bool,
}

#[repr(C)]
pub struct SaveFileDialogParams<'a> {
    pub name_field_string_value: BorrowedArray<'a, u8>,
}
