use std::ffi::CString;

use desktop_common::ffi_utils::BorrowedStrPtr;
use log::warn;

use crate::gtk::events::{EventHandler, FileChooserResponse, NotificationShownEvent, RequestId};

#[allow(clippy::enum_variant_names)]
pub enum AsyncEventResult {
    UrlOpenResponse {
        request_id: RequestId,
        error: Option<anyhow::Error>,
    },
    FileChooserResponse {
        request_id: RequestId,
        result: anyhow::Result<CString>,
    },
    NotificationClosed {},
    NotificationShown {
        request_id: RequestId,
        result: anyhow::Result<u32>,
    },
}

impl AsyncEventResult {
    pub fn send_as_event(self, event_handler: EventHandler) {
        match self {
            Self::UrlOpenResponse { request_id, error } => {
                if let Some(e) = error {
                    warn!("Error trying to open URL for {request_id:?}: {e}");
                }
            }
            Self::FileChooserResponse { request_id, result } => {
                let send = |newline_separated_files| {
                    let response = FileChooserResponse {
                        request_id,
                        newline_separated_files,
                    };
                    event_handler(&response.into());
                };
                match result {
                    Ok(files) => {
                        send(files.as_c_str().into());
                    }
                    Err(e) => {
                        warn!("{e}");
                        send(BorrowedStrPtr::null());
                    }
                }
            }
            Self::NotificationClosed {} => {}
            Self::NotificationShown { request_id, result } => {
                let notification_id = result.unwrap_or_else(|e| {
                    warn!("{e}");
                    0
                });
                let event = NotificationShownEvent {
                    request_id,
                    notification_id,
                };
                event_handler(&event.into());
            }
        }
    }
}
