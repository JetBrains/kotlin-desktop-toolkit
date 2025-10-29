use smithay_client_toolkit::seat::pointer::CursorIcon;

use crate::linux::pointer_shapes_api::PointerShape;

impl From<PointerShape> for Option<CursorIcon> {
    fn from(value: PointerShape) -> Self {
        match value {
            PointerShape::Hidden => None,
            PointerShape::Default => Some(CursorIcon::Default),
            PointerShape::ContextMenu => Some(CursorIcon::ContextMenu),
            PointerShape::Help => Some(CursorIcon::Help),
            PointerShape::Pointer => Some(CursorIcon::Pointer),
            PointerShape::Progress => Some(CursorIcon::Progress),
            PointerShape::Wait => Some(CursorIcon::Wait),
            PointerShape::Cell => Some(CursorIcon::Cell),
            PointerShape::Crosshair => Some(CursorIcon::Crosshair),
            PointerShape::Text => Some(CursorIcon::Text),
            PointerShape::VerticalText => Some(CursorIcon::VerticalText),
            PointerShape::Alias => Some(CursorIcon::Alias),
            PointerShape::Copy => Some(CursorIcon::Copy),
            PointerShape::Move => Some(CursorIcon::Move),
            PointerShape::NoDrop => Some(CursorIcon::NoDrop),
            PointerShape::NotAllowed => Some(CursorIcon::NotAllowed),
            PointerShape::Grab => Some(CursorIcon::Grab),
            PointerShape::Grabbing => Some(CursorIcon::Grabbing),
            PointerShape::EResize => Some(CursorIcon::EResize),
            PointerShape::NResize => Some(CursorIcon::NResize),
            PointerShape::NeResize => Some(CursorIcon::NeResize),
            PointerShape::NwResize => Some(CursorIcon::NwResize),
            PointerShape::SResize => Some(CursorIcon::SResize),
            PointerShape::SeResize => Some(CursorIcon::SeResize),
            PointerShape::SwResize => Some(CursorIcon::SwResize),
            PointerShape::WResize => Some(CursorIcon::WResize),
            PointerShape::EwResize => Some(CursorIcon::EwResize),
            PointerShape::NsResize => Some(CursorIcon::NsResize),
            PointerShape::NeswResize => Some(CursorIcon::NeswResize),
            PointerShape::NwseResize => Some(CursorIcon::NwseResize),
            PointerShape::ColResize => Some(CursorIcon::ColResize),
            PointerShape::RowResize => Some(CursorIcon::RowResize),
            PointerShape::AllScroll => Some(CursorIcon::AllScroll),
            PointerShape::ZoomIn => Some(CursorIcon::ZoomIn),
            PointerShape::ZoomOut => Some(CursorIcon::ZoomOut),
        }
    }
}
