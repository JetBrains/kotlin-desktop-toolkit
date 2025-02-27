use crate::common::BorrowedStrPtr;

use super::{application_api::AppState, window_api::WindowId};

#[repr(C)]
#[derive(Debug, Default)]
// For the invalid (missing) value, all values are 0
pub struct TextRange {
    pub(crate) location: usize,
    pub(crate) length: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct TextChangedOperation<'a> {
    pub(crate) window_id: WindowId,
    pub(crate) text: BorrowedStrPtr<'a>,
    pub(crate) replacement_range: TextRange,
    //composition_range: TextRange,
    //composition_committed_range: TextRange,
    //composition_selected_range: TextRange,
}

#[repr(C)]
#[derive(Debug)]
pub struct TextCommandOperation<'a> {
    pub(crate) window_id: WindowId,
    pub(crate) command: BorrowedStrPtr<'a>,
}

#[repr(C)]
#[derive(Debug)]
pub enum TextOperation<'a> {
    TextChanged(TextChangedOperation<'a>),
    TextCommand(TextCommandOperation<'a>),
}

// return true if operation was handled
pub type TextOperationHandler = extern "C" fn(&TextOperation) -> bool;

#[repr(C)]
#[derive(Debug)]
pub struct GetSelectedTextRangeOperation {
    pub(crate) window_id: WindowId,
}

#[repr(C)]
#[derive(Debug)]
pub struct FirstRectForCharacterRangeOperation {
    pub(crate) window_id: WindowId,
    pub(crate) location: usize,
    pub(crate) length: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct GetSelectedTextRangeResult {
    pub(crate) location: usize,
    pub(crate) length: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct FirstRectForCharacterRangeResult {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TextContextHandler {
    get_selected_range: extern "C" fn(GetSelectedTextRangeOperation) -> GetSelectedTextRangeResult,
    first_rect_for_character_range: extern "C" fn(FirstRectForCharacterRangeOperation) -> FirstRectForCharacterRangeResult,
}

impl TextChangedOperation<'_> {
    pub(crate) fn get_result(self) -> bool {
        AppState::with(|state| {
            let operation = TextOperation::TextChanged(self);
            (state.text_operation_handler)(&operation)
        })
    }
}

impl TextCommandOperation<'_> {
    pub(crate) fn get_result(self) -> bool {
        AppState::with(|state| {
            let operation = TextOperation::TextCommand(self);
            (state.text_operation_handler)(&operation)
        })
    }
}

impl GetSelectedTextRangeOperation {
    pub(crate) fn get_result(self) -> GetSelectedTextRangeResult {
        AppState::with(|state| (state.text_context_handler.get_selected_range)(self))
    }
}

impl FirstRectForCharacterRangeOperation {
    pub(crate) fn get_result(self) -> FirstRectForCharacterRangeResult {
        AppState::with(|state| (state.text_context_handler.first_rect_for_character_range)(self))
    }
}
