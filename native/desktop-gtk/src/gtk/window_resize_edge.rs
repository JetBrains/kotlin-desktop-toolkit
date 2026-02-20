use crate::gtk::window_resize_edge_api::WindowResizeEdge;
use gtk4::gdk as gdk4;

impl From<WindowResizeEdge> for gdk4::SurfaceEdge {
    fn from(value: WindowResizeEdge) -> Self {
        match value {
            WindowResizeEdge::Top => Self::North,
            WindowResizeEdge::Bottom => Self::South,
            WindowResizeEdge::Left => Self::West,
            WindowResizeEdge::TopLeft => Self::NorthWest,
            WindowResizeEdge::BottomLeft => Self::SouthWest,
            WindowResizeEdge::Right => Self::East,
            WindowResizeEdge::TopRight => Self::NorthEast,
            WindowResizeEdge::BottomRight => Self::SouthEast,
        }
    }
}
