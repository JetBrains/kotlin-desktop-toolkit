use std::ffi::CStr;

use crate::common::BorrowedStrPtr;

use super::{application_api::AppState, window_api::WindowId};

#[repr(C)]
#[derive(Debug, Default)]
// For the invalid (missing) value, all values are 0
pub struct TextRange {
    start_offset_inclusive: i64,
    end_offset_inclusive: i64,
}

#[repr(C)]
#[derive(Debug)]
pub struct TextChangedOperation<'a> {
    window_id: WindowId,
    text: BorrowedStrPtr<'a>,
    //composition_range: TextRange,
    //composition_committed_range: TextRange,
    //composition_selected_range: TextRange,
    //replacement_range: TextRange,
}

#[repr(C)]
#[derive(Debug)]
pub struct TextCommandOperation<'a> {
    window_id: WindowId,
    command: BorrowedStrPtr<'a>,
}

#[repr(C)]
#[derive(Debug)]
pub enum TextOperation<'a> {
    TextCommand(TextCommandOperation<'a>),
    TextChanged(TextChangedOperation<'a>),
}

// return true if operation was handled
pub type TextOperationHandler = extern "C" fn(&TextOperation) -> bool;

pub(crate) fn handle_text_changed_operation(window_id: WindowId, text: BorrowedStrPtr) -> anyhow::Result<bool> {
    AppState::with(|state| {
        let operation = TextOperation::TextChanged(TextChangedOperation {
            window_id,
            text,
            //composition_range: TextRange::default(),
            //composition_committed_range: TextRange::default(),
            //composition_selected_range: TextRange::default(),
            //replacement_range: TextRange::default(),
        });
        Ok((state.text_operation_handler)(&operation))
    })
}

pub(crate) fn handle_text_command_operation(window_id: WindowId, command: &'static CStr) -> anyhow::Result<bool> {
    AppState::with(|state| {
        let operation = TextOperation::TextCommand(TextCommandOperation {
            window_id,
            command: BorrowedStrPtr::new(command),
        });
        Ok((state.text_operation_handler)(&operation))
    })
}
