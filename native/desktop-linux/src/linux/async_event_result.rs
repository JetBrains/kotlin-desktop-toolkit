use std::ffi::CString;

use desktop_common::ffi_utils::BorrowedStrPtr;
use log::error;

use crate::linux::events::{EventHandler, FileChooserResponse, RequestId, WindowId};

pub enum AsyncEventResult {
    UrlOpenResponse {
        request_id: RequestId,
        error: Option<anyhow::Error>,
    },
    FileChooserResponse {
        request_id: RequestId,
        window_id: WindowId,
        result: anyhow::Result<CString>,
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
                match result {
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
