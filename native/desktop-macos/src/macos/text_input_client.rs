#![allow(dead_code)]
#![allow(unused_variables)]
use std::ptr::NonNull;

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

impl TextInputClient {
    pub fn has_marked_text(&self) -> bool {
        let ret = self.marked_range().is_some();
        debug!("hasMarkedText: {ret}");
        ret
    }

    pub fn marked_range(&self) -> Option<NSRange> {
        debug!("markedRange");
        self.ivars().marked_text_range.get() // TODO
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
        let ivars = self.ivars();
        let (ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        debug!(
            "setMarkedText, marked_text={:?}, string={:?}, selected_range={:?}, replacement_range={:?}",
            ns_attributed_string, text, selected_range, replacement_range
        );
        if !self.try_handle_current_key_down_event() {
            ivars.marked_text_range.set(Some(selected_range));
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
        }
        Ok(true)
    }

    pub fn unmark_text(&self) -> anyhow::Result<bool> {
        debug!("unmarkText");
        self.ivars().current_key_down_event.set(None);
        self.ivars().marked_text_range.set(None);
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

        if self.try_handle_current_key_down_event() {
            Ok(true)
        } else {
            (self.on_insert_text)(borrow_ns_string(&text));
            ivars.marked_text_range.set(None);
            Ok(true)
        }
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

    pub fn key_down(&self, ns_event: &NSEvent) -> anyhow::Result<bool> {
        debug!("keyDown start: {ns_event:?}");
        let ivars = self.ivars();
        let key_event_info = unpack_key_event(ns_event)?;
        let had_marked_text = self.has_marked_text();
        ivars.current_key_down_event.set(Some(key_event_info));
        debug!("keyDown, calling interpretKeyEvents");
        // TODO: call only if we have ActiveTextInput set
        // or expose `interpretKeyEvents` which will be called by the app when we have ActiveTextInput set
        unsafe {
            let key_events = NSArray::arrayWithObject(ns_event);
            self.interpretKeyEvents(&key_events);
        };
        let handled = if let Some(key_event_info) = ivars.current_key_down_event.take() {
            if had_marked_text || self.has_marked_text() /* to handle the initial IME navigation key */ {
                debug!("keyDown: has/had marked text, not forwarding");
                true
            } else {
                debug!("keyDown: forwarding");
                let handled = self.handle_event(&Event::new_key_down_event(&key_event_info));
                debug!("keyDown: handled = {handled}");
                handled
            }
        } else {
            debug!("keyDown: handled by interpretKeyEvents, not forwarding");
            true
        };
        debug!("keyDown end");
        Ok(handled)
    }

    pub fn try_handle_current_key_down_event(&self) -> bool {
        if self.has_marked_text() {
            return false;
        }
        if let Some(key_info) = self.ivars().current_key_down_event.take() {
            let e = Event::new_key_down_event(&key_info);
            if self.handle_event(&e) {
                if let Some(input_context) = self.inputContext() {
                    input_context.discardMarkedText();
                }
                return true;
            }
        }
        false
    }

    pub fn do_command(&self, selector: Sel) -> anyhow::Result<bool> {
        let s = selector.name();
        if s == c"noop:" {
            debug!("Ignoring the noop: selector, forwarding the raw event");
            return Ok(false);
        }
        debug!("do_command_by_selector: {s:?}");
        if !self.try_handle_current_key_down_event() {
            (self.on_do_command)(BorrowedStrPtr::new(s));
        }
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
