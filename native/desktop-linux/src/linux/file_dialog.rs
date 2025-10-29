use std::ffi::CString;

use ashpd::desktop::file_chooser;
use smithay_client_toolkit::{reexports::client::protocol::wl_surface::WlSurface, shell::WaylandSurface as _};

use crate::linux::{
    application::Application,
    async_event_result::AsyncEventResult,
    events::{RequestId, WindowId},
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
    fn create_open_request(&self, open_params: &OpenFileDialogParams) -> anyhow::Result<file_chooser::OpenFileRequest> {
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

    fn create_save_request(&self, save_params: &SaveFileDialogParams) -> anyhow::Result<file_chooser::SaveFileRequest> {
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

async fn show_open_file_dialog_impl(wl_surface: &WlSurface, request: file_chooser::OpenFileRequest) -> anyhow::Result<CString> {
    let identifier = ashpd::WindowIdentifier::from_wayland(wl_surface).await;
    let open_file_request = request.identifier(identifier);
    let response = open_file_request.send().await.unwrap().response();
    convert_file_chooser_response(response)
}

async fn show_save_file_dialog_impl(wl_surface: &WlSurface, request: file_chooser::SaveFileRequest) -> anyhow::Result<CString> {
    let identifier = ashpd::WindowIdentifier::from_wayland(wl_surface).await;
    let open_file_request = request.identifier(identifier);
    let response = open_file_request.send().await.unwrap().response();
    convert_file_chooser_response(response)
}

impl Application {
    fn get_wl_surface(&self, window_id: WindowId) -> anyhow::Result<WlSurface> {
        Ok(self.get_window(window_id)?.window.wl_surface().clone())
    }

    pub fn show_open_file_dialog(
        &mut self,
        window_id: WindowId,
        common_params: &CommonFileDialogParams,
        open_params: &OpenFileDialogParams,
    ) -> anyhow::Result<RequestId> {
        let wl_surface = self.get_wl_surface(window_id)?;
        let request = common_params.create_open_request(open_params)?;
        Ok(self.run_async(|request_id| async move {
            let result = show_open_file_dialog_impl(&wl_surface, request).await;
            AsyncEventResult::FileChooserResponse {
                request_id,
                window_id,
                result,
            }
        }))
    }

    pub fn show_save_file_dialog(
        &mut self,
        window_id: WindowId,
        common_params: &CommonFileDialogParams,
        save_params: &SaveFileDialogParams,
    ) -> anyhow::Result<RequestId> {
        let wl_surface = self.get_wl_surface(window_id)?;
        let request = common_params.create_save_request(save_params)?;
        Ok(self.run_async(|request_id| async move {
            let result = show_save_file_dialog_impl(&wl_surface, request).await;
            AsyncEventResult::FileChooserResponse {
                request_id,
                window_id,
                result,
            }
        }))
    }
}
