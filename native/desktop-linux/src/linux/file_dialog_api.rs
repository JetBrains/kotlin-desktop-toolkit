use desktop_common::ffi_utils::BorrowedUtf8;

#[repr(C)]
pub struct CommonFileDialogParams<'a> {
    pub modal: bool,
    pub title: BorrowedUtf8<'a>,
    pub accept_label: BorrowedUtf8<'a>,
    pub current_folder: BorrowedUtf8<'a>,
}

impl CommonFileDialogParams<'_> {
    pub(crate) fn get_title(&self) -> anyhow::Result<Option<&str>> {
        self.title.get_optional("CommonFileDialogParams.title")
    }

    pub(crate) fn get_accept_label(&self) -> anyhow::Result<Option<&str>> {
        self.accept_label.get_optional("CommonFileDialogParams.accept_label")
    }

    pub(crate) fn get_current_folder(&self) -> anyhow::Result<Option<&str>> {
        self.current_folder.get_optional("CommonFileDialogParams.current_folder")
    }
}

#[repr(C)]
pub struct OpenFileDialogParams {
    pub select_directories: bool,
    pub allows_multiple_selection: bool,
}

#[repr(C)]
pub struct SaveFileDialogParams<'a> {
    pub name_field_string_value: BorrowedUtf8<'a>,
}

impl SaveFileDialogParams<'_> {
    pub(crate) fn get_name_field_string_value(&self) -> anyhow::Result<Option<&str>> {
        self.name_field_string_value
            .get_optional("SaveFileDialogParams.name_field_string_value")
    }
}
