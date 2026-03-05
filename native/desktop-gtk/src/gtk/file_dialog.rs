use std::ffi::CString;

use ashpd::{WindowIdentifier, desktop::file_chooser};

use crate::gtk::{
    file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams},
    string_utils::join_str_iter,
};

impl OpenFileDialogParams {
    fn apply(&self, request: file_chooser::OpenFileRequest) -> file_chooser::OpenFileRequest {
        request.directory(self.select_directories).multiple(self.allows_multiple_selection)
    }
}

impl SaveFileDialogParams<'_> {
    fn apply(&self, mut request: file_chooser::SaveFileRequest) -> anyhow::Result<file_chooser::SaveFileRequest> {
        if let Some(name_field_string_value) = self.name_field_string_value.as_optional_str()? {
            request = request.current_name(name_field_string_value);
        }
        Ok(request)
    }
}

impl CommonFileDialogParams<'_> {
    pub fn create_open_request(&self, open_params: &OpenFileDialogParams) -> anyhow::Result<file_chooser::OpenFileRequest> {
        let mut request = file_chooser::SelectedFiles::open_file().modal(self.modal);
        if let Some(title) = self.title.as_optional_str()? {
            request = request.title(title);
        }
        if let Some(accept_label) = self.accept_label.as_optional_str()? {
            request = request.accept_label(accept_label);
        }
        if let Some(current_folder) = self.current_folder.as_optional_str()? {
            request = request.current_folder(current_folder)?;
        }
        Ok(open_params.apply(request))
    }

    pub fn create_save_request(&self, save_params: &SaveFileDialogParams) -> anyhow::Result<file_chooser::SaveFileRequest> {
        let mut request = file_chooser::SelectedFiles::save_file().modal(self.modal);
        if let Some(title) = self.title.as_optional_str()? {
            request = request.title(title);
        }
        if let Some(accept_label) = self.accept_label.as_optional_str()? {
            request = request.accept_label(accept_label);
        }
        if let Some(current_folder) = self.current_folder.as_optional_str()? {
            request = request.current_folder(current_folder)?;
        }
        save_params.apply(request)
    }
}

fn convert_file_chooser_response(response: Result<file_chooser::SelectedFiles, ashpd::Error>) -> anyhow::Result<CString> {
    let files = response?;
    let newline_separated_files = join_str_iter(files.uris().iter().map(ashpd::url::Url::as_str), "\r\n");
    Ok(CString::new(newline_separated_files)?)
}

pub async fn show_open_file_dialog_impl(
    identifier: Option<WindowIdentifier>,
    request: file_chooser::OpenFileRequest,
) -> anyhow::Result<CString> {
    let open_file_request = request.identifier(identifier);
    let response = open_file_request.send().await?.response();
    convert_file_chooser_response(response)
}

pub async fn show_save_file_dialog_impl(
    identifier: Option<WindowIdentifier>,
    request: file_chooser::SaveFileRequest,
) -> anyhow::Result<CString> {
    let open_file_request = request.identifier(identifier);
    let response = open_file_request.send().await?.response();
    convert_file_chooser_response(response)
}
