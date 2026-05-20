use smithay_client_toolkit::seat::pointer::CursorIcon;

use crate::linux::pointer_shapes_api::PointerShape;

impl PointerShape {
    #[must_use]
    pub const fn to_wayland_cursor_icon(self) -> Option<CursorIcon> {
        match self {
            Self::Hidden => None,
            Self::Default => Some(CursorIcon::Default),
            Self::ContextMenu => Some(CursorIcon::ContextMenu),
            Self::Help => Some(CursorIcon::Help),
            Self::Pointer => Some(CursorIcon::Pointer),
            Self::Progress => Some(CursorIcon::Progress),
            Self::Wait => Some(CursorIcon::Wait),
            Self::Cell => Some(CursorIcon::Cell),
            Self::Crosshair => Some(CursorIcon::Crosshair),
            Self::Text => Some(CursorIcon::Text),
            Self::VerticalText => Some(CursorIcon::VerticalText),
            Self::Alias => Some(CursorIcon::Alias),
            Self::Copy => Some(CursorIcon::Copy),
            Self::Move => Some(CursorIcon::Move),
            Self::NoDrop => Some(CursorIcon::NoDrop),
            Self::NotAllowed => Some(CursorIcon::NotAllowed),
            Self::Grab => Some(CursorIcon::Grab),
            Self::Grabbing => Some(CursorIcon::Grabbing),
            Self::EResize => Some(CursorIcon::EResize),
            Self::NResize => Some(CursorIcon::NResize),
            Self::NeResize => Some(CursorIcon::NeResize),
            Self::NwResize => Some(CursorIcon::NwResize),
            Self::SResize => Some(CursorIcon::SResize),
            Self::SeResize => Some(CursorIcon::SeResize),
            Self::SwResize => Some(CursorIcon::SwResize),
            Self::WResize => Some(CursorIcon::WResize),
            Self::EwResize => Some(CursorIcon::EwResize),
            Self::NsResize => Some(CursorIcon::NsResize),
            Self::NeswResize => Some(CursorIcon::NeswResize),
            Self::NwseResize => Some(CursorIcon::NwseResize),
            Self::ColResize => Some(CursorIcon::ColResize),
            Self::RowResize => Some(CursorIcon::RowResize),
            Self::AllScroll => Some(CursorIcon::AllScroll),
            Self::ZoomIn => Some(CursorIcon::ZoomIn),
            Self::ZoomOut => Some(CursorIcon::ZoomOut),
        }
    }
}
