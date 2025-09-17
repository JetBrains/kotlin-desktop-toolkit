use log::error;

use crate::linux::events::{EventHandler, RequestId};

pub enum AsyncEventResult {
    UrlOpenResponse {
        request_id: RequestId,
        error: Option<ashpd::Error>,
    },
}

impl AsyncEventResult {
    pub fn send_as_event(self, _event_handler: EventHandler) {
        match self {
            Self::UrlOpenResponse { request_id, error } => {
                if let Some(e) = error {
                    error!("Error trying to open URL for {request_id:?}: {e}");
                }
            }
        }
    }
}
