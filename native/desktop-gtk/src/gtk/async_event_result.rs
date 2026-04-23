use desktop_common::ffi_utils::BorrowedUtf8;
use log::warn;

use crate::gtk::events::{EventHandler, FileChooserResponse, NotificationShownEvent, RequestId};

#[allow(clippy::enum_variant_names)]
pub enum AsyncEventResult {
    FileChooserResponse {
        request_id: RequestId,
        result: anyhow::Result<Option<String>>,
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
            Self::FileChooserResponse { request_id, result } => {
                let send = |newline_separated_files| {
                    let response = FileChooserResponse {
                        request_id,
                        newline_separated_files,
                    };
                    event_handler(&response.into());
                };
                match result {
                    Ok(Some(files)) => {
                        send(BorrowedUtf8::new(&files));
                    }
                    Ok(None) => {
                        send(BorrowedUtf8::null());
                    }
                    Err(e) => {
                        warn!("{e}");
                        send(BorrowedUtf8::null());
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
