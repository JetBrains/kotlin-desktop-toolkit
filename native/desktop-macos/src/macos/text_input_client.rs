use std::{cell::Cell, ptr::NonNull};

use anyhow::{Context, bail};
use log::{debug, warn};
use objc2::{
    DefinedClass, MainThreadMarker,
    rc::Retained,
    runtime::{AnyObject, Sel},
};
use objc2_app_kit::{NSBeep, NSScreen};
use objc2_foundation::{
    NSArray, NSAttributedString, NSAttributedStringKey, NSNotFound, NSPoint, NSRange, NSRangePointer, NSRect, NSString, NSUInteger,
};

use super::{application_api::MyNSApplication, string::borrow_ns_string, window_api::WindowPtr};
use crate::{
    geometry::{LogicalPoint, LogicalRect},
    macos::{screen::NSScreenExts, string::copy_nonnull_to_ns_string, window::Window},
};
use desktop_common::{ffi_utils::BorrowedStrPtr, logger::ffi_boundary};

#[repr(C)]
#[derive(Debug)]
pub struct TextRange {
    pub location: usize,
    pub length: usize,
}

impl Default for TextRange {
    fn default() -> Self {
        Self {
            #[allow(clippy::cast_sign_loss)] // isize to usize
            location: NSNotFound as usize,
            length: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct InsertTextArgs<'a> {
    pub text: BorrowedStrPtr<'a>,
    pub replacement_range: TextRange,
}

#[repr(C)]
pub struct SetMarkedTextArgs<'a> {
    pub text: BorrowedStrPtr<'a>,
    pub selected_range: TextRange,
    pub replacement_range: TextRange,
}

#[repr(C)]
#[derive(Debug)]
pub struct FirstRectForCharacterRangeArgs {
    range_in: TextRange,
    actual_range_out: TextRange,
    first_rect_out: LogicalRect,
}

pub type HasMarkedTextCallback = extern "C" fn() -> bool;
pub type MarkedRangeCallback = extern "C" fn(range_out: &mut TextRange);
pub type SelectedRangeCallback = extern "C" fn(range_out: &mut TextRange);
pub type InsertTextCallback = extern "C" fn(args: InsertTextArgs);
pub type DoCommandCallback = extern "C" fn(command: BorrowedStrPtr) -> bool;
pub type UnmarkTextCallback = extern "C" fn();
pub type SetMarkedTextCallback = extern "C" fn(args: SetMarkedTextArgs);
pub type FirstRectForCharacterRangeCallback = extern "C" fn(args: &mut FirstRectForCharacterRangeArgs);
pub type CharacterIndexForPoint = extern "C" fn(LogicalPoint) -> usize;

#[repr(C)]
pub struct AttributedStringForRangeResult<'a> {
    string: BorrowedStrPtr<'a>,
    actual_range: TextRange,
}

pub type AttributedStringForRangeCallback = extern "C" fn(range: TextRange) -> AttributedStringForRangeResult<'static>;
pub type FreeAttributedStringCallback = extern "C" fn();

#[repr(C)]
pub struct TextInputClient {
    pub has_marked_text: HasMarkedTextCallback,
    pub marked_range: MarkedRangeCallback,
    pub selected_range: SelectedRangeCallback,
    pub insert_text: InsertTextCallback,
    pub do_command: DoCommandCallback,
    pub unmark_text: UnmarkTextCallback,
    pub set_marked_text: SetMarkedTextCallback,

    // this two is kinda special because it returns Jvm allocated string
    // and we need to free it somehow
    pub attributed_string_for_range: AttributedStringForRangeCallback,
    pub free_attributed_string_for_range: FreeAttributedStringCallback,
    //
    pub first_rect_for_character_range: FirstRectForCharacterRangeCallback,
    pub character_index_for_point: CharacterIndexForPoint,
}

pub(crate) struct TextInputClientHandler {
    pub client: TextInputClient,
    pub last_commnad_handler_result: Cell<Option<bool>>,
}

// https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/TextEditing/Tasks/TextViewTask.html
impl TextInputClientHandler {
    pub const fn new(text_input_client: TextInputClient) -> Self {
        Self {
            client: text_input_client,
            last_commnad_handler_result: Cell::new(None),
        }
    }

    pub fn has_marked_text(&self) -> bool {
        let ret = (self.client.has_marked_text)();
        debug!("hasMarkedText() -> {ret:?}");
        ret
    }

    pub fn marked_range(&self) -> NSRange {
        let mut result = TextRange::default();
        (self.client.marked_range)(&mut result);
        debug!("markedRange() -> {result:?}");
        result.into()
    }

    pub fn selected_range(&self) -> NSRange {
        let mut result = TextRange::default();
        (self.client.selected_range)(&mut result);
        debug!("selectedRange() -> {result:?}");
        result.into()
    }

    pub fn set_marked_text(&self, string: &AnyObject, selected_range: NSRange, replacement_range: NSRange) -> anyhow::Result<()> {
        let (_ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        debug!(
            "setMarkedText(marked_text={text:?}, selected_range={:?}, replacement_range={:?})",
            PrintableNSRange(selected_range),
            PrintableNSRange(replacement_range)
        );
        (self.client.set_marked_text)(SetMarkedTextArgs {
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
        Ok(())
    }

    pub fn unmark_text(&self) {
        debug!("unmarkText()");
        (self.client.unmark_text)();
    }

    #[allow(clippy::unused_self)]
    pub fn valid_attributes_for_marked_text(&self) -> Retained<NSArray<NSAttributedStringKey>> {
        debug!("validAttributesForMarkedText() -> []");
        //        let v = vec![
        //            NSString::from_str("NSFont"),
        //            NSString::from_str("NSUnderline"),
        //            NSString::from_str("NSColor"),
        //            NSString::from_str("NSBackgroundColor"),
        //            NSString::from_str("NSUnderlineColor"),
        //            NSString::from_str("NSMarkedClauseSegment"),
        //            NSString::from_str("NSLanguage"),
        //            NSString::from_str("NSTextInputReplacementRangeAttributeName"),
        //            NSString::from_str("NSGlyphInfo"),
        //            NSString::from_str("NSTextAlternatives"),
        //            NSString::from_str("NSTextInsertionUndoable"),
        //        ];
        //        NSArray::from_retained_slice(&v)
        NSArray::new()
    }

    pub fn attributed_substring_for_proposed_range(
        &self,
        range: NSRange,
        actual_range: NSRangePointer,
    ) -> anyhow::Result<Option<Retained<NSAttributedString>>> {
        let result = (self.client.attributed_string_for_range)(range.into());
        let ns_string = result.string.as_non_null().map(copy_nonnull_to_ns_string).transpose()?;
        write_to_range_ptr(actual_range, result.actual_range.into());
        (self.client.free_attributed_string_for_range)();
        debug!(
            "attributedSubstringForProposedRange(range={:?}) -> {ns_string:?}",
            PrintableNSRange(range)
        );
        Ok(ns_string.map(|s| NSAttributedString::from_nsstring(&s)))
    }

    pub fn insert_text(&self, string: &AnyObject, replacement_range: NSRange) -> anyhow::Result<()> {
        let (_ns_attributed_string, text) = get_maybe_attributed_string(string)?;
        (self.client.insert_text)(InsertTextArgs {
            text: borrow_ns_string(&text),
            replacement_range: replacement_range.into(),
        });
        debug!(
            "insertText(string={text:?}, replacement_range={:?})",
            PrintableNSRange(replacement_range)
        );
        Ok(())
    }

    pub fn first_rect_for_character_range(&self, range: NSRange, actual_range: NSRangePointer) -> anyhow::Result<NSRect> {
        let mtm = MainThreadMarker::new().unwrap();

        let mut args = FirstRectForCharacterRangeArgs {
            range_in: range.into(),
            actual_range_out: TextRange::default(),
            first_rect_out: LogicalRect::default(),
        };
        (self.client.first_rect_for_character_range)(&mut args);

        write_to_range_ptr(actual_range, args.actual_range_out.into());
        let screen_height = NSScreen::primary(mtm)?.height();

        let result_rect = args.first_rect_out.as_macos_coords(screen_height);
        debug!("firstRectForCharacterRange(range={:?}) -> {result_rect:?}", PrintableNSRange(range));
        Ok(result_rect)
    }

    pub fn character_index_for_point(&self, point: NSPoint) -> anyhow::Result<NSUInteger> {
        let mtm = MainThreadMarker::new().unwrap();

        let screen_height = NSScreen::primary(mtm)?.height();
        let logical_point = LogicalPoint::from_macos_coords(point, screen_height);

        let index = (self.client.character_index_for_point)(logical_point);
        debug!("characterIndexForPoint(point = {point:?}) -> {index:?}");
        Ok(index)
    }

    pub fn do_command(&self, selector: Sel) {
        debug!("doCommand: {selector:?}");
        let was_handled = (self.client.do_command)(BorrowedStrPtr::new(selector.name()));
        let prev_value = self.last_commnad_handler_result.replace(Some(was_handled));
        if prev_value.is_some() {
            warn!("Overwrite some doCommand result: {prev_value:?}");
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_context_handle_current_event(window_ptr: WindowPtr) -> bool {
    ffi_boundary("text_input_context_handle_current_event", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        let input_context = window.root_view.inputContext();
        let mtm = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        let current_event = app.currentEvent().context("Should be called from event handler")?;
        debug!("input_context.handleEvent start {current_event:?}");
        let result = match input_context {
            Some(input_context) => {
                let command_result_cell = &window.root_view.ivars().text_input_client_handler.last_commnad_handler_result;
                let prev_value = command_result_cell.replace(None);
                if prev_value.is_some() {
                    warn!("Overwrite some doCommand result: {prev_value:?}");
                }
                let was_event_handled = unsafe { input_context.handleEvent(&current_event) };
                if was_event_handled {
                    command_result_cell.replace(None).unwrap_or(true)
                } else {
                    false
                }
            }
            None => false,
        };
        debug!("input_context.handleEvent end retuned: {result:?}");
        Ok(result)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_context_discard_marked_text(window_ptr: WindowPtr) {
    ffi_boundary("text_input_context_discard_marked_text", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        let input_context = window.root_view.inputContext().context("No Input Context")?;
        input_context.discardMarkedText();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_context_invalidate_character_coordinates(window_ptr: WindowPtr) {
    ffi_boundary("text_input_context_invalidate_character_coordinates", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        let input_context = window.root_view.inputContext().context("No InputContext")?;
        input_context.invalidateCharacterCoordinates();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_context_not_found_offset() -> isize {
    ffi_boundary("text_input_context_not_found_offset", || Ok(NSNotFound))
}

#[unsafe(no_mangle)]
pub extern "C" fn text_input_context_beep() {
    ffi_boundary("text_input_context_beep", || {
        unsafe {
            NSBeep();
        }
        Ok(())
    });
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

fn write_to_range_ptr(range_ptr: NSRangePointer, range: NSRange) {
    let mut range_ptr = NonNull::new(range_ptr);
    match &mut range_ptr {
        Some(range_ptr) => unsafe {
            range_ptr.write(range);
        },
        None => {
            warn!("Got Null as actual_range which is unexpected");
        }
    }
}

impl From<TextRange> for NSRange {
    fn from(value: TextRange) -> Self {
        Self {
            location: value.location,
            length: value.length,
        }
    }
}

impl From<NSRange> for TextRange {
    fn from(value: NSRange) -> Self {
        Self {
            location: value.location,
            length: value.length,
        }
    }
}

#[allow(clippy::cast_sign_loss)] // isize to usize
pub const NOT_FOUND_NS_RANGE: NSRange = NSRange {
    location: NSNotFound as usize,
    length: 0,
};

#[repr(transparent)]
struct PrintableNSRange(NSRange);

impl std::fmt::Debug for PrintableNSRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 == NOT_FOUND_NS_RANGE {
            write!(f, "NSRange::NotFound")
        } else {
            write!(f, "{:?}", self.0)
        }
    }
}
