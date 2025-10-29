use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;

use crate::linux::window_resize_edge_api::WindowResizeEdge;

impl From<WindowResizeEdge> for ResizeEdge {
    fn from(value: WindowResizeEdge) -> Self {
        match value {
            WindowResizeEdge::None => Self::None,
            WindowResizeEdge::Top => Self::Top,
            WindowResizeEdge::Bottom => Self::Bottom,
            WindowResizeEdge::Left => Self::Left,
            WindowResizeEdge::TopLeft => Self::TopLeft,
            WindowResizeEdge::BottomLeft => Self::BottomLeft,
            WindowResizeEdge::Right => Self::Right,
            WindowResizeEdge::TopRight => Self::TopRight,
            WindowResizeEdge::BottomRight => Self::BottomRight,
        }
    }
}
