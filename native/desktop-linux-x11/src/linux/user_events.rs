use crate::linux::application_api::RenderingMode;
use crate::linux::events::WindowId;
use crate::linux::geometry::{LogicalRect, LogicalSize};

pub enum UserEvents {
    CreateWindow {
        window_id: WindowId,
        rect: LogicalRect,
        min_size: Option<LogicalSize>,
        title: String,
        app_id: String,
        prefer_client_side_decoration: bool,
        rendering_mode: RenderingMode,
    },
    Exit,
    RunOnEventLoop(extern "C" fn()),
    // ClipboardReceived {
    //     serial: i32,
    //     content: ClipboardContent,
    // },
}
