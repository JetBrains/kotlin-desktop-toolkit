use winit_core::window::ResizeDirection;

use crate::linux::window_resize_edge_api::WindowResizeEdge;

impl From<WindowResizeEdge> for Option<ResizeDirection> {
    fn from(value: WindowResizeEdge) -> Self {
        match value {
            WindowResizeEdge::None => None,
            WindowResizeEdge::Top => Some(ResizeDirection::North),
            WindowResizeEdge::Bottom => Some(ResizeDirection::South),
            WindowResizeEdge::Left => Some(ResizeDirection::West),
            WindowResizeEdge::TopLeft => Some(ResizeDirection::NorthWest),
            WindowResizeEdge::BottomLeft => Some(ResizeDirection::SouthWest),
            WindowResizeEdge::Right => Some(ResizeDirection::East),
            WindowResizeEdge::TopRight => Some(ResizeDirection::NorthEast),
            WindowResizeEdge::BottomRight => Some(ResizeDirection::SouthWest),
        }
    }
}
