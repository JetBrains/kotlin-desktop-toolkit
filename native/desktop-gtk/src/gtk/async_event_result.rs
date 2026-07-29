use desktop_common::ffi_utils::BorrowedUtf8;
use log::warn;

use crate::gtk::events::{Event, EventHandler, FileChooserResponse, RequestId};

#[allow(clippy::enum_variant_names)]
pub enum AsyncEventResult {
    FileChooserResponse(anyhow::Result<Option<String>>),
}

impl AsyncEventResult {
    pub fn send_as_event(self, event_handler: EventHandler, request_id: RequestId) {
        match self {
            Self::FileChooserResponse(result) => {
                let send = |newline_separated_files| {
                    let response = FileChooserResponse {
                        request_id,
                        newline_separated_files,
                    };
                    event_handler(&Event::from(response));
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
        }
    }
}
