use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowResizeEdge {
    /// Nothing is being dragged.
    None,
    /// The top edge is being dragged.
    Top,
    /// The bottom edge is being dragged.
    Bottom,
    /// The left edge is being dragged.
    Left,
    /// The top left corner is being dragged.
    TopLeft,
    /// The bottom left corner is being dragged.
    BottomLeft,
    /// The right edge is being dragged.
    Right,
    /// The top right corner is being dragged.
    TopRight,
    /// The bottom right corner is being dragged.
    BottomRight,
}

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
