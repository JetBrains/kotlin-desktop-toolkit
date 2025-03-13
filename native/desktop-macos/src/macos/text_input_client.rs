#![allow(dead_code)]
#![allow(unused_variables)]
use std::{cell::Cell, ptr::NonNull};

use anyhow::Context;
use log::debug;
use objc2::{
    MainThreadMarker,
    rc::Retained,
    runtime::{AnyObject, Sel},
};
use objc2_app_kit::{NSEvent, NSTextInputContext};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSPoint, NSRange, NSRangePointer, NSRect, NSSize, NSString, NSUInteger,
};

use crate::{common::BorrowedStrPtr, logger::ffi_boundary, macos::{events::Event, keyboard::unpack_key_event}};

use super::{application_api::MyNSApplication, string::borrow_ns_string};

#[repr(C)]
#[derive(Debug, Default)]
 // For the invalid (missing) value, all values are 0
pub struct TextRange {
    pub location: usize,
    pub length: usize,
}

//#[repr(C)]
//#[derive(Debug)]
//pub struct TextChangedOperation<'a> {
//    pub text: BorrowedStrPtr<'a>,
//    //pub composition_range: TextRange,
//    //pub composition_committed_range: TextRange,
//    //pub composition_selected_range: TextRange,
//    //pub replacement_range: TextRange,
//}

#[repr(C)]
pub struct SetMarkedTextOperation<'a> {
    pub text: BorrowedStrPtr<'a>,
    pub selected_range: TextRange,
    pub replacement_range: TextRange,
}

pub type OnInsertText = extern "C" fn(text: BorrowedStrPtr);
pub type OnDoCommand = extern "C" fn(command: BorrowedStrPtr);
pub type OnUnmarkText = extern "C" fn();
pub type OnSetMarkedText = extern "C" fn(data: SetMarkedTextOperation);

#[repr(C)]
pub struct TextInputClient {
    pub on_insert_text: OnInsertText,
    pub on_do_command: OnDoCommand,
    pub on_unmark_text: OnUnmarkText,
    pub on_set_marked_text: OnSetMarkedText,
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_context_handle_current_event() -> bool {
    ffi_boundary("text_input_context_handle_current_event", || {
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let current_event = app.currentEvent().context("Should be called from event handler")?;
        let result = if let Some(input_context) = unsafe { NSTextInputContext::currentInputContext(mtm) } {
            unsafe { input_context.handleEvent(&current_event) }
        } else {
            false
        };
        Ok(result)
    })
}

const DEFAULT_NS_RANGE: NSRange = NSRange { location: 0, length: 0 };
const DEFAULT_NS_RECT: NSRect = NSRect::new(NSPoint::new(0f64, 0f64), NSSize::new(0f64, 0f64));

pub(crate) struct TextInputClientHandler {
    client: TextInputClient,
    handled_key_down_event: Cell<bool>,
    marked_text_range: Cell<Option<NSRange>>,
}

impl TextInputClient {
//    pub fn send_event_to_input_context(&self, ns_event: &NSEvent, input_context: &NSTextInputContext) -> bool {
//        if !unsafe { input_context.handleEvent(ns_event) } {
//            false
//        } else {
//            self.handled_key_down_event.get()
//        }
//    }

    pub fn has_marked_text(&self) -> bool {
        let ret = self.marked_range().is_some();
        debug!("hasMarkedText: {ret}");
        ret
    }

    pub fn marked_range(&self) -> Option<NSRange> {
        debug!("markedRange");
        //self.marked_text_range.get() // TODO
        None // TODO
    }

    pub fn selected_range(&self) -> NSRange {
        debug!("selectedRange");
        DEFAULT_NS_RANGE // TODO
    }

    pub fn set_marked_text(
        &self,
        string: &AnyObject,
        selected_range: NSRange,
        replacement_range: NSRange,
    ) -> anyhow::Result<bool> {
        let (ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        debug!(
            "setMarkedText, marked_text={:?}, string={:?}, selected_range={:?}, replacement_range={:?}",
            ns_attributed_string, text, selected_range, replacement_range
        );
        (self.on_set_marked_text)(SetMarkedTextOperation {
            text: borrow_ns_string(&text),
            selected_range: TextRange {
                location: selected_range.location,
                length: selected_range.length,
            },
            replacement_range: TextRange {
                location: replacement_range.location,
                length: replacement_range.length,
            },
        });
//        self.handled_key_down_event.set(true);
//        self.marked_text_range.set(Some(selected_range));
        Ok(true)
    }

    pub fn unmark_text(&self) -> anyhow::Result<bool> {
        debug!("unmarkText");
//        self.handled_key_down_event.set(true);
//        self.marked_text_range.set(None);
        (self.on_unmark_text)();
        Ok(true)
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

    pub fn attributed_substring_for_proposed_range(
        &self,
        range: NSRange,
        actual_range: NSRangePointer,
    ) -> Option<Retained<NSAttributedString>> {
        let actual_range = NonNull::new(actual_range);
        debug!(
            "attributedSubstringForProposedRange, range={:?}, actual_range={:?}",
            range,
            actual_range.map(|r| unsafe { r.read() })
        );
        None // TODO
    }

    pub fn insert_text(&self, string: &AnyObject, replacement_range: NSRange) -> anyhow::Result<bool> {
        let (ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        debug!(
            "insertText, marked_text={:?}, string={:?}, replacement_range={:?}",
            ns_attributed_string, text, replacement_range
        );

        (self.on_insert_text)(borrow_ns_string(&text));
//        self.handled_key_down_event.set(true);
//        self.marked_text_range.set(None);
        Ok(true)
    }

    pub fn first_rect_for_character_range(&self, range: NSRange, actual_range: NSRangePointer) -> anyhow::Result<NSRect> {
        let actual_range = NonNull::new(actual_range);
        debug!(
            "firstRectForCharacterRange: range={:?}, actual_range={:?}",
            range,
            actual_range.map(|r| unsafe { r.read() })
        );
        Ok(DEFAULT_NS_RECT) // TODO
    }

    pub fn character_index_for_point(&self, point: NSPoint) -> anyhow::Result<NSUInteger> {
        debug!("characterIndexForPoint: {:?}", point);
        Ok(0) // TODO
    }

    pub fn do_command(&self, selector: Sel) -> anyhow::Result<bool> {
        let s = selector.name();
        if s == c"noop:" {
            debug!("Ignoring the noop: selector, forwarding the raw event");
            return Ok(false);
        }
        debug!("do_command_by_selector: {s:?}");
        (self.on_do_command)(BorrowedStrPtr::new(s));
        Ok(true)
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
