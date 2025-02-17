use std::ffi::CStr;

use crate::common::ConstStrPtr;

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
pub struct TextChangedOperation {
    window_id: WindowId,
    text: ConstStrPtr,
    //composition_range: TextRange,
    //composition_committed_range: TextRange,
    //composition_selected_range: TextRange,
    //replacement_range: TextRange,
}

#[repr(C)]
#[derive(Debug)]
pub struct TextCommandOperation {
    window_id: WindowId,
    command: ConstStrPtr,
}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum TextOperation {
    TextCommand(TextCommandOperation),
    TextChanged(TextChangedOperation),
}

// return true if operation was handled
pub type TextOperationHandler = extern "C" fn(&TextOperation) -> bool;

pub(crate) fn handle_text_changed_operation(window_id: WindowId, text: &CStr) -> anyhow::Result<bool> {
    let handled = AppState::with(|state| {
        let operation = TextOperation::TextChanged(TextChangedOperation {
            window_id,
            text: text.as_ptr(),
            //composition_range: TextRange::default(),
            //composition_committed_range: TextRange::default(),
            //composition_selected_range: TextRange::default(),
            //replacement_range: TextRange::default(),
        });
        Ok((state.text_operation_handler)(&operation))
    });
    handled
}

pub(crate) fn handle_text_command_operation(window_id: WindowId, command: &'static CStr) -> anyhow::Result<bool> {
    let handled = AppState::with(|state| {
        let operation = TextOperation::TextCommand(TextCommandOperation {
            window_id,
            command: command.as_ptr(),
        });
        Ok((state.text_operation_handler)(&operation))
    });
    handled
}
