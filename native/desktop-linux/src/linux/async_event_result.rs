use std::ffi::CString;

use ashpd::desktop::file_chooser::SelectedFiles;
use desktop_common::ffi_utils::BorrowedStrPtr;
use log::error;

use crate::linux::{
    events::{EventHandler, FileChooserResponse, RequestId, WindowId},
    string_utils::join_str_iter,
};

pub enum AsyncEventResult {
    UrlOpenResponse {
        request_id: RequestId,
        error: Option<ashpd::Error>,
    },
    FileChooserResponse {
        request_id: RequestId,
        window_id: WindowId,
        result: Result<ashpd::desktop::file_chooser::SelectedFiles, ashpd::Error>,
    },
}

impl AsyncEventResult {
    pub fn send_as_event(self, event_handler: EventHandler) {
        match self {
            Self::UrlOpenResponse { request_id, error } => {
                if let Some(e) = error {
                    error!("Error trying to open URL for {request_id:?}: {e}");
                }
            }
            Self::FileChooserResponse {
                request_id,
                window_id,
                result,
            } => {
                let send = |newline_separated_files| {
                    let response = FileChooserResponse {
                        window_id,
                        request_id,
                        newline_separated_files,
                    };
                    event_handler(&response.into())
                };
                match convert_file_chooser_response(result) {
                    Ok(files) => {
                        send(files.as_c_str().into());
                    }
                    Err(e) => {
                        error!("{e}");
                        send(BorrowedStrPtr::null());
                    }
                }
            }
        }
    }
}

fn convert_file_chooser_response(response: Result<SelectedFiles, ashpd::Error>) -> anyhow::Result<CString> {
    let files = response?;
    let newline_separated_files = join_str_iter(files.uris().iter().map(ashpd::url::Url::as_str), "\r\n");
    Ok(CString::new(newline_separated_files)?)
}
