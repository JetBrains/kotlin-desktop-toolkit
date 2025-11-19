use crate::linux::pointer_shapes_api::PointerShape;
use winit_core::cursor::{Cursor, CursorIcon};

impl From<PointerShape> for Option<Cursor> {
    fn from(value: PointerShape) -> Self {
        match value {
            PointerShape::Hidden => None,
            PointerShape::Default => Some(Cursor::Icon(CursorIcon::Default)),
            PointerShape::ContextMenu => Some(Cursor::Icon(CursorIcon::ContextMenu)),
            PointerShape::Help => Some(Cursor::Icon(CursorIcon::Help)),
            PointerShape::Pointer => Some(Cursor::Icon(CursorIcon::Pointer)),
            PointerShape::Progress => Some(Cursor::Icon(CursorIcon::Progress)),
            PointerShape::Wait => Some(Cursor::Icon(CursorIcon::Wait)),
            PointerShape::Cell => Some(Cursor::Icon(CursorIcon::Cell)),
            PointerShape::Crosshair => Some(Cursor::Icon(CursorIcon::Crosshair)),
            PointerShape::Text => Some(Cursor::Icon(CursorIcon::Text)),
            PointerShape::VerticalText => Some(Cursor::Icon(CursorIcon::VerticalText)),
            PointerShape::Alias => Some(Cursor::Icon(CursorIcon::Alias)),
            PointerShape::Copy => Some(Cursor::Icon(CursorIcon::Copy)),
            PointerShape::Move => Some(Cursor::Icon(CursorIcon::Move)),
            PointerShape::NoDrop => Some(Cursor::Icon(CursorIcon::NoDrop)),
            PointerShape::NotAllowed => Some(Cursor::Icon(CursorIcon::NotAllowed)),
            PointerShape::Grab => Some(Cursor::Icon(CursorIcon::Grab)),
            PointerShape::Grabbing => Some(Cursor::Icon(CursorIcon::Grabbing)),
            PointerShape::EResize => Some(Cursor::Icon(CursorIcon::EResize)),
            PointerShape::NResize => Some(Cursor::Icon(CursorIcon::NResize)),
            PointerShape::NeResize => Some(Cursor::Icon(CursorIcon::NeResize)),
            PointerShape::NwResize => Some(Cursor::Icon(CursorIcon::NwResize)),
            PointerShape::SResize => Some(Cursor::Icon(CursorIcon::SResize)),
            PointerShape::SeResize => Some(Cursor::Icon(CursorIcon::SeResize)),
            PointerShape::SwResize => Some(Cursor::Icon(CursorIcon::SwResize)),
            PointerShape::WResize => Some(Cursor::Icon(CursorIcon::WResize)),
            PointerShape::EwResize => Some(Cursor::Icon(CursorIcon::EwResize)),
            PointerShape::NsResize => Some(Cursor::Icon(CursorIcon::NsResize)),
            PointerShape::NeswResize => Some(Cursor::Icon(CursorIcon::NeswResize)),
            PointerShape::NwseResize => Some(Cursor::Icon(CursorIcon::NwseResize)),
            PointerShape::ColResize => Some(Cursor::Icon(CursorIcon::ColResize)),
            PointerShape::RowResize => Some(Cursor::Icon(CursorIcon::RowResize)),
            PointerShape::AllScroll => Some(Cursor::Icon(CursorIcon::AllScroll)),
            PointerShape::ZoomIn => Some(Cursor::Icon(CursorIcon::ZoomIn)),
            PointerShape::ZoomOut => Some(Cursor::Icon(CursorIcon::ZoomOut)),
        }
    }
}
