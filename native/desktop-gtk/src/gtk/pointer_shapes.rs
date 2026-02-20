use crate::gtk::pointer_shapes_api::PointerShape;
use gtk4::gdk as gdk4;

impl From<PointerShape> for Option<gdk4::Cursor> {
    fn from(value: PointerShape) -> Self {
        match value {
            PointerShape::Hidden => None,
            PointerShape::Default => gdk4::Cursor::from_name("default", None),
            PointerShape::ContextMenu => gdk4::Cursor::from_name("context-menu", None),
            PointerShape::Help => gdk4::Cursor::from_name("help", None),
            PointerShape::Pointer => gdk4::Cursor::from_name("pointer", None),
            PointerShape::Progress => gdk4::Cursor::from_name("progress", None),
            PointerShape::Wait => gdk4::Cursor::from_name("wait", None),
            PointerShape::Cell => gdk4::Cursor::from_name("cell", None),
            PointerShape::Crosshair => gdk4::Cursor::from_name("crosshair", None),
            PointerShape::Text => gdk4::Cursor::from_name("text", None),
            PointerShape::VerticalText => gdk4::Cursor::from_name("vertical-text", None),
            PointerShape::Alias => gdk4::Cursor::from_name("alias", None),
            PointerShape::Copy => gdk4::Cursor::from_name("copy", None),
            PointerShape::Move => gdk4::Cursor::from_name("move", None),
            PointerShape::NoDrop => gdk4::Cursor::from_name("no-drop", None),
            PointerShape::NotAllowed => gdk4::Cursor::from_name("not-allowed", None),
            PointerShape::Grab => gdk4::Cursor::from_name("grab", None),
            PointerShape::Grabbing => gdk4::Cursor::from_name("grabbing", None),
            PointerShape::EResize => gdk4::Cursor::from_name("e-resize", None),
            PointerShape::NResize => gdk4::Cursor::from_name("n-resize", None),
            PointerShape::NeResize => gdk4::Cursor::from_name("ne-resize", None),
            PointerShape::NwResize => gdk4::Cursor::from_name("nw-resize", None),
            PointerShape::SResize => gdk4::Cursor::from_name("s-resize", None),
            PointerShape::SeResize => gdk4::Cursor::from_name("se-resize", None),
            PointerShape::SwResize => gdk4::Cursor::from_name("sw-resize", None),
            PointerShape::WResize => gdk4::Cursor::from_name("w-resize", None),
            PointerShape::EwResize => gdk4::Cursor::from_name("ew-resize", None),
            PointerShape::NsResize => gdk4::Cursor::from_name("ns-resize", None),
            PointerShape::NeswResize => gdk4::Cursor::from_name("nesw-resize", None),
            PointerShape::NwseResize => gdk4::Cursor::from_name("nwse-resize", None),
            PointerShape::ColResize => gdk4::Cursor::from_name("col-resize", None),
            PointerShape::RowResize => gdk4::Cursor::from_name("row-resize", None),
            PointerShape::AllScroll => gdk4::Cursor::from_name("all-scroll", None),
            PointerShape::ZoomIn => gdk4::Cursor::from_name("zoom-in", None),
            PointerShape::ZoomOut => gdk4::Cursor::from_name("zoom-out", None),
        }
    }
}
