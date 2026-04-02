use crate::gtk::events::WindowId;
use crate::gtk::geometry::LogicalPoint;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DragAndDropAction {
    None = 0b0000_0000,

    Copy = 0b0000_0001,

    Move = 0b0000_0010,
    // TODO?: Ask
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct DragAndDropActions(pub u8);

#[repr(C)]
#[derive(Debug)]
pub struct DragAndDropQueryData {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataSource {
    Clipboard,
    DragAndDrop,
    PrimarySelection,
}
