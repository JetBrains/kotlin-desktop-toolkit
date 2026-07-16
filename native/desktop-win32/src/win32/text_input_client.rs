use desktop_common::ffi_utils::{BorrowedArray, BorrowedUtf8};

use windows::Win32::Foundation::RECT;

use super::geometry::{LogicalPoint, LogicalRect, LogicalSize};

/// cbindgen:ignore
const NOT_FOUND: usize = usize::MAX;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub location: usize,
    pub length: usize,
}

impl TextRange {
    pub(crate) const fn none() -> Self {
        Self {
            location: NOT_FOUND,
            length: 0,
        }
    }

    pub(crate) const fn into_option(self) -> Option<Self> {
        if self.location == NOT_FOUND { None } else { Some(self) }
    }
}

#[repr(C)]
pub struct InsertTextArgs<'a> {
    pub text: BorrowedUtf8<'a>,
}

#[repr(C)]
pub struct SetMarkedTextArgs<'a> {
    pub text: BorrowedUtf8<'a>,
    pub selected_range: TextRange,
    pub underlines: BorrowedArray<'a, UnderlineSegment>,
}

#[repr(C)]
pub struct CaretRectArgs {
    pub range_in: TextRange,
    pub rect_out: LogicalRect,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderlineSegment {
    pub range: TextRange,
    pub style: UnderlineStyle,
    pub target_clause: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnderlineStyle {
    Solid,
    Dotted,
    Thick,
}

pub type SelectedRangeCallback = extern "C" fn(range_out: &mut TextRange);
pub type CaretRectCallback = extern "C" fn(args: &mut CaretRectArgs);
pub type InsertTextCallback = extern "C" fn(args: InsertTextArgs);
pub type SetMarkedTextCallback = extern "C" fn(args: SetMarkedTextArgs);
pub type UnmarkTextCallback = extern "C" fn();
pub type DiscardMarkedTextCallback = extern "C" fn();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TextInputClient {
    pub selected_range: SelectedRangeCallback,
    pub caret_rect: CaretRectCallback,
    pub insert_text: InsertTextCallback,
    pub set_marked_text: SetMarkedTextCallback,
    pub unmark_text: UnmarkTextCallback,
    pub discard_marked_text: DiscardMarkedTextCallback,
}

impl TextInputClient {
    pub(crate) fn selected_range(self) -> Option<TextRange> {
        let mut out = TextRange::none();
        (self.selected_range)(&mut out);
        out.into_option()
    }

    pub(crate) fn caret_rect(self, range: TextRange) -> LogicalRect {
        let mut args = CaretRectArgs {
            range_in: range,
            rect_out: LogicalRect {
                origin: LogicalPoint::new(0.0, 0.0),
                size: LogicalSize::new(0.0, 0.0),
            },
        };
        (self.caret_rect)(&mut args);
        args.rect_out
    }

    pub(crate) fn insert_text(self, text: &str) {
        (self.insert_text)(InsertTextArgs {
            text: BorrowedUtf8::new(text),
        });
    }

    pub(crate) fn set_marked_text(self, text: &str, selected_range: Option<TextRange>, underlines: &[UnderlineSegment]) {
        (self.set_marked_text)(SetMarkedTextArgs {
            text: BorrowedUtf8::new(text),
            selected_range: selected_range.unwrap_or_else(TextRange::none),
            underlines: BorrowedArray::from_slice(underlines),
        });
    }

    pub(crate) fn unmark_text(self) {
        (self.unmark_text)();
    }

    pub(crate) fn discard_marked_text(self) {
        (self.discard_marked_text)();
    }
}

pub(crate) fn client_logical_to_physical_rect(rect: LogicalRect, scale: f32) -> RECT {
    let top_left = rect.origin.to_physical(scale);
    let bottom_right = LogicalPoint::new(rect.origin.x.0 + rect.size.width.0, rect.origin.y.0 + rect.size.height.0).to_physical(scale);
    RECT {
        left: top_left.x.0,
        top: top_left.y.0,
        right: bottom_right.x.0,
        bottom: bottom_right.y.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_range_round_trips() {
        assert_eq!(TextRange::none().into_option(), None);
        let range = TextRange { location: 4, length: 2 };
        assert_eq!(range.into_option(), Some(range));
    }

    #[test]
    fn logical_caret_rect_scales_both_corners() {
        let rect = LogicalRect {
            origin: LogicalPoint::new(10.25, 5.25),
            size: LogicalSize::new(3.5, 4.5),
        };
        let physical = client_logical_to_physical_rect(rect, 1.5);
        assert_eq!((physical.left, physical.top, physical.right, physical.bottom), (15, 8, 21, 15));
    }
}
