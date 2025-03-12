use crate::common::BorrowedStrPtr;

use super::{
    events::{CallbackUserData, KeyDownEvent},
    window_api::WindowId,
};

#[repr(C)]
#[derive(Debug, Default, PartialEq, Eq)]
// For the invalid (missing) value, all values are 0
pub struct TextRange {
    pub location: usize,
    pub length: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct TextChangedOperation<'a> {
    pub window_id: WindowId,
    pub original_event: Option<&'a KeyDownEvent<'a>>,
    pub text: BorrowedStrPtr<'a>,
//    pub composition_range: TextRange,
//    pub composition_committed_range: TextRange,
//    pub composition_selected_range: TextRange,
}

#[repr(C)]
#[derive(Debug)]
pub struct TextCommandOperation<'a> {
    pub window_id: WindowId,
    pub original_event: Option<&'a KeyDownEvent<'a>>,
    pub command: BorrowedStrPtr<'a>,
}

#[repr(C)]
#[derive(Debug)]
pub struct SetMarkedTextOperation<'a> {
    pub window_id: WindowId,
    pub text: BorrowedStrPtr<'a>,
    pub selected_range: TextRange,
    pub replacement_range: TextRange,
}

#[repr(C)]
#[derive(Debug)]
pub struct UnmarkTextOperation {
    pub window_id: WindowId,
}

#[repr(C)]
#[derive(Debug)]
pub enum TextOperation<'a> {
    TextCommand(TextCommandOperation<'a>),
    TextChanged(TextChangedOperation<'a>),
    SetMarkedText(SetMarkedTextOperation<'a>),
    UnmarkText(UnmarkTextOperation),
}

// return true if operation was handled
pub type TextOperationHandler = extern "C" fn(&TextOperation, CallbackUserData) -> bool;
