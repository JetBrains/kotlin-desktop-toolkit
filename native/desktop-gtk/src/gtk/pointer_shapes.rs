use crate::gtk::pointer_shapes_api::PointerShape;
use gtk4::gdk as gdk4;

impl PointerShape {
    #[must_use]
    pub fn to_gtk_cursor(self) -> Option<gdk4::Cursor> {
        match self {
            Self::Hidden => gdk4::Cursor::from_name("none", None),
            Self::Default => gdk4::Cursor::from_name("default", None),
            Self::ContextMenu => gdk4::Cursor::from_name("context-menu", None),
            Self::Help => gdk4::Cursor::from_name("help", None),
            Self::Pointer => gdk4::Cursor::from_name("pointer", None),
            Self::Progress => gdk4::Cursor::from_name("progress", None),
            Self::Wait => gdk4::Cursor::from_name("wait", None),
            Self::Cell => gdk4::Cursor::from_name("cell", None),
            Self::Crosshair => gdk4::Cursor::from_name("crosshair", None),
            Self::Text => gdk4::Cursor::from_name("text", None),
            Self::VerticalText => gdk4::Cursor::from_name("vertical-text", None),
            Self::Alias => gdk4::Cursor::from_name("alias", None),
            Self::Copy => gdk4::Cursor::from_name("copy", None),
            Self::Move => gdk4::Cursor::from_name("move", None),
            Self::NoDrop => gdk4::Cursor::from_name("no-drop", None),
            Self::NotAllowed => gdk4::Cursor::from_name("not-allowed", None),
            Self::Grab => gdk4::Cursor::from_name("grab", None),
            Self::Grabbing => gdk4::Cursor::from_name("grabbing", None),
            Self::EResize => gdk4::Cursor::from_name("e-resize", None),
            Self::NResize => gdk4::Cursor::from_name("n-resize", None),
            Self::NeResize => gdk4::Cursor::from_name("ne-resize", None),
            Self::NwResize => gdk4::Cursor::from_name("nw-resize", None),
            Self::SResize => gdk4::Cursor::from_name("s-resize", None),
            Self::SeResize => gdk4::Cursor::from_name("se-resize", None),
            Self::SwResize => gdk4::Cursor::from_name("sw-resize", None),
            Self::WResize => gdk4::Cursor::from_name("w-resize", None),
            Self::EwResize => gdk4::Cursor::from_name("ew-resize", None),
            Self::NsResize => gdk4::Cursor::from_name("ns-resize", None),
            Self::NeswResize => gdk4::Cursor::from_name("nesw-resize", None),
            Self::NwseResize => gdk4::Cursor::from_name("nwse-resize", None),
            Self::ColResize => gdk4::Cursor::from_name("col-resize", None),
            Self::RowResize => gdk4::Cursor::from_name("row-resize", None),
            Self::AllScroll => gdk4::Cursor::from_name("all-scroll", None),
            Self::ZoomIn => gdk4::Cursor::from_name("zoom-in", None),
            Self::ZoomOut => gdk4::Cursor::from_name("zoom-out", None),
        }
    }
}
