use crate::linux::application_api::RenderingMode;
use crate::linux::events::WindowId;
use crate::linux::geometry::LogicalRect;

pub enum UserEvents {
    CreateWindow {
        window_id: WindowId,
        rect: LogicalRect,
        title: String,
        app_id: String,
        prefer_client_side_decoration: bool,
        rendering_mode: RenderingMode,
    },
    Exit,
    RunOnEventLoop(extern "C" fn()),
}
