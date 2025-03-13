#![allow(dead_code)]
#![allow(unused_variables)]
use std::{cell::Cell, ptr::NonNull};

use anyhow::bail;
use log::debug;
use objc2::{
    rc::Retained,
    runtime::{AnyObject, Sel},
};
use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSTextInputContext};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSPoint, NSRange, NSRangePointer, NSRect, NSSize, NSString, NSUInteger,
};

use crate::{common::BorrowedStrPtr, macos::keyboard::key_codes};

use super::string::borrow_ns_string;

#[repr(C)]
#[derive(Debug, Default)]
// For the invalid (missing) value, all values are 0
pub struct TextRange {
    pub location: usize,
    pub length: usize,
}

pub type OnDoCommand = extern "C" fn(command: BorrowedStrPtr) -> bool;

#[repr(C)]
#[derive(Debug)]
pub struct OnInsertTextArgs<'a> {
    pub text: BorrowedStrPtr<'a>,
    //pub composition_range: TextRange,
    //pub composition_committed_range: TextRange,
    //pub composition_selected_range: TextRange,
    //pub replacement_range: TextRange,
}
pub type OnInsertText = extern "C" fn(args: OnInsertTextArgs);

#[repr(C)]
pub struct OnSetMarkedTextArgs<'a> {
    pub text: BorrowedStrPtr<'a>,
    pub selected_range: TextRange,
    pub replacement_range: TextRange,
}
pub type OnSetMarkedText = extern "C" fn(args: OnSetMarkedTextArgs);

pub type OnUnmarkText = extern "C" fn();

#[repr(C)]
pub struct TextInputClient {
    pub on_insert_text: OnInsertText,
    pub on_do_command: OnDoCommand,
    pub on_unmark_text: OnUnmarkText,
    pub on_set_marked_text: OnSetMarkedText,
}

const DEFAULT_NS_RANGE: NSRange = NSRange { location: 0, length: 0 };
const DEFAULT_NS_RECT: NSRect = NSRect::new(NSPoint::new(0f64, 0f64), NSSize::new(0f64, 0f64));

pub(crate) struct TextInputClientHandler {
    pub client: TextInputClient,
    pub do_command_handled_event: Cell<bool>,
    pub marked_text_range: Cell<Option<NSRange>>,
}

impl TextInputClientHandler {
    pub fn new(text_input_client: TextInputClient) -> Self {
        TextInputClientHandler {
            client: text_input_client,
            do_command_handled_event: Cell::new(false),
            marked_text_range: Cell::new(None),
        }
    }

    fn send_event_to_input_context(&self, ns_event: &NSEvent, input_context: &NSTextInputContext) -> bool {
        self.do_command_handled_event.set(true);
        if unsafe { input_context.handleEvent(ns_event) } {
            self.do_command_handled_event.get()
        } else {
            false
        }
    }

    pub fn on_key_down<F>(
        &self,
        ns_event: &NSEvent,
        input_context: &Option<Retained<NSTextInputContext>>,
        emit_key_down: F,
    ) -> anyhow::Result<bool>
    where
        F: FnOnce(&NSEvent) -> anyhow::Result<bool>,
    {
        assert!(unsafe { ns_event.r#type() } == NSEventType::KeyDown);
        debug!("keyDown start: {ns_event:?}");
        let handled: bool = if let Some(input_context) = input_context {
            let modifiers = unsafe { ns_event.modifierFlags() };
            if self.has_marked_text() || is_ime_navigation_key(ns_event) && !modifiers.contains(NSEventModifierFlags::Control) {
                self.send_event_to_input_context(ns_event, input_context) || emit_key_down(ns_event)?
            } else {
                emit_key_down(ns_event)? || self.send_event_to_input_context(ns_event, input_context)
            }
        } else {
            debug!("keyDown: input_context not found");
            emit_key_down(ns_event)?
        };

        debug!("keyDown end: handled = {handled}");
        Ok(handled)
    }

    pub fn has_marked_text(&self) -> bool {
        let ret = self.marked_range().is_some();
        debug!("hasMarkedText: {ret}");
        ret
    }

    pub fn marked_range(&self) -> Option<NSRange> {
        debug!("markedRange");
        self.marked_text_range.get()
    }

    #[allow(clippy::unused_self)]
    pub fn selected_range(&self) -> NSRange {
        debug!("selectedRange");
        DEFAULT_NS_RANGE // TODO
    }

    pub fn set_marked_text(&self, string: &AnyObject, selected_range: NSRange, replacement_range: NSRange) -> anyhow::Result<bool> {
        let (ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        debug!(
            "setMarkedText, marked_text={:?}, string={:?}, selected_range={:?}, replacement_range={:?}",
            ns_attributed_string, text, selected_range, replacement_range
        );
        (self.client.on_set_marked_text)(OnSetMarkedTextArgs {
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
        self.marked_text_range.set(Some(selected_range));
        Ok(true)
    }

    pub fn unmark_text(&self) -> bool {
        debug!("unmarkText");
        self.marked_text_range.set(None);
        (self.client.on_unmark_text)();
        true
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

        (self.client.on_insert_text)(OnInsertTextArgs {
            text: borrow_ns_string(&text),
        });
        self.marked_text_range.set(None);
        Ok(true)
    }

    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    pub fn first_rect_for_character_range(&self, range: NSRange, actual_range: NSRangePointer) -> anyhow::Result<NSRect> {
        let actual_range = NonNull::new(actual_range);
        debug!(
            "firstRectForCharacterRange: range={:?}, actual_range={:?}",
            range,
            actual_range.map(|r| unsafe { r.read() })
        );
        Ok(DEFAULT_NS_RECT) // TODO
    }

    #[allow(clippy::unused_self, clippy::unnecessary_wraps)]
    pub fn character_index_for_point(&self, point: NSPoint) -> anyhow::Result<NSUInteger> {
        debug!("characterIndexForPoint: {:?}", point);
        Ok(0) // TODO
    }

    pub fn do_command(&self, selector: Sel) -> bool {
        let s = selector.name();
        if s == c"noop:" {
            debug!("Ignoring the noop: selector, forwarding the raw event");
            self.do_command_handled_event.set(false);
            return false;
        }
        debug!("do_command_by_selector: {s:?}");
        if !(self.client.on_do_command)(BorrowedStrPtr::new(s)) {
            self.do_command_handled_event.set(false);
        }
        true
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
        bail!("unexpected text {string:?}")
    }
}

fn is_ime_navigation_key(ns_event: &NSEvent) -> bool {
    // TODO: improve heuristic, e.g. Ctrl+J in Fleet Shortcut binding window
    [
        key_codes::VK_Escape,
        key_codes::VK_LeftArrow,
        key_codes::VK_RightArrow,
        key_codes::VK_DownArrow,
        key_codes::VK_UpArrow,
    ]
    .contains(&unsafe { ns_event.keyCode() })
}
