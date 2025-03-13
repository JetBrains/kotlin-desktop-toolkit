#![allow(dead_code)]
#![allow(unused_variables)]
use std::ptr::NonNull;

use anyhow::Context;
use log::debug;
use objc2::{rc::Retained, runtime::{AnyObject, Sel}, MainThreadMarker};
use objc2_app_kit::NSTextInputContext;
use objc2_foundation::{NSArray, NSAttributedString, NSAttributedStringKey, NSPoint, NSRange, NSRangePointer, NSRect, NSSize, NSString, NSUInteger};

use crate::{common::BorrowedStrPtr, logger::ffi_boundary};

use super::{application_api::MyNSApplication, string::borrow_ns_string};

pub type OnInsertText = extern "C" fn(text: BorrowedStrPtr);
pub type OnDoCommand = extern "C" fn(command: BorrowedStrPtr);

#[repr(C)]
pub struct TextInputClient {
    pub on_insert_text: OnInsertText,
    pub on_do_command: OnDoCommand,
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_context_handle_current_event() -> bool {
    ffi_boundary("text_input_context_handle_current_event", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let current_event = app.currentEvent().context("Should be called from event handler")?;
        let input_context = unsafe {
            NSTextInputContext::currentInputContext(mtm)
        };
        let result = match input_context {
            Some(input_context) => {
                unsafe {
                    input_context.handleEvent(&current_event)
                }
            }
            None => {
                false
            }
        };
        Ok(result)
    })
}

impl TextInputClient {
    pub fn has_marked_text(&self) -> bool {
        // TODO
        false
    }

    pub fn marked_range(&self) -> NSRange {
        // TODO
        NSRange { location: 0, length: 0 }  // TODO
    }

    pub fn selected_range(&self) -> NSRange {
        NSRange { location: 0, length: 0 }  // TODO
    }

    pub fn set_marked_text(&self,
                           string: &AnyObject,
                           selected_range: NSRange,
                           replacement_range: NSRange) {

    }

    pub fn unmark_text(&self) {

    }

    pub fn valid_attributes_for_marked_text(&self) -> Retained<NSArray<NSAttributedStringKey>> {
        debug!("validAttributesForMarkedText");
        let v = vec![
            NSString::from_str("NSFont"),
            NSString::from_str("NSUnderline"),
            NSString::from_str("NSColor"),
            NSString::from_str("NSBackgroundColor"),
            NSString::from_str("NSUnderlineColor"),
            NSString::from_str("NSMarkedClauseSegment"),
            NSString::from_str("NSLanguage"),
            NSString::from_str("NSTextInputReplacementRangeAttributeName"),
            NSString::from_str("NSGlyphInfo"),
            NSString::from_str("NSTextAlternatives"),
            NSString::from_str("NSTextInsertionUndoable"),
        ];
        NSArray::from_retained_slice(&v)
    }

    pub fn attributed_substring_for_proposed_range(&self, range: NSRange, actual_range: NSRangePointer) -> Option<Retained<NSAttributedString>> {
        None
    }

    pub fn insert_text(&self, string: &AnyObject, replacement_range: NSRange) -> anyhow::Result<()> {
        let (_ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        (self.on_insert_text)(borrow_ns_string(&text));
        Ok(())
    }

    pub fn first_rect_for_character_range(&self, range: NSRange, actual_range: NSRangePointer) -> NSRect {
        let actual_range = NonNull::new(actual_range);
        debug!("firstRectForCharacterRange: range={:?}, actual_range={:?}", range, actual_range.map(|r| unsafe { r.read() }));
        NSRect::new(NSPoint::new(0f64, 0f64), NSSize::new(0f64, 0f64))  // TODO
    }

    pub fn character_index_for_point(&self, point: NSPoint) -> NSUInteger {
        0
    }

    pub fn do_command(&self, command: Sel) {
        let command_name = command.name();
        (self.on_do_command)(BorrowedStrPtr::new(command_name))
    }
}



fn get_maybe_attributed_string(string: &AnyObject) -> Result<(Option<&NSAttributedString>, Retained<NSString>), anyhow::Error> {
    if let Some(ns_attributed_string) = string.downcast_ref::<NSAttributedString>() {
        let text = ns_attributed_string.string();
        Ok((Some(ns_attributed_string), text))
    } else if let Some(text) = string.downcast_ref::<NSString>() {
        Ok((None, text.into()))
    } else {
        // This method is guaranteed to get either a `NSString` or a `NSAttributedString`.
        panic!("unexpected text {string:?}")
    }
}

