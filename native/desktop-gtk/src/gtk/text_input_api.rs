use crate::gtk::geometry::LogicalRect;
use desktop_common::ffi_utils::BorrowedStrPtr;
use enumflags2::{BitFlags, bitflags};

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextInputContextHint {
    None,
    WordCompletion,
    Spellcheck,
    Lowercase,
    UppercaseChars,
    UppercaseWords,
    UppercaseSentences,
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct TextInputContextHintBitflag(pub u32);

impl From<TextInputContextHint> for TextInputContextHintBitflag {
    fn from(value: TextInputContextHint) -> Self {
        Self(BitFlags::from_flag(value).bits_c())
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TextInputContext<'a> {
    pub surrounding_text: BorrowedStrPtr<'a>,
    pub cursor_codepoint_offset: u16,
    pub selection_start_codepoint_offset: u16,
    pub hints: TextInputContextHintBitflag,
    pub content_purpose: TextInputContentPurpose,
    pub cursor_rectangle: LogicalRect,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum TextInputContentPurpose {
    /// default input, allowing all characters
    Normal,
    /// allow only alphabetic characters
    Alpha,
    /// allow only digits
    Digits,
    /// input a number (including decimal separator and sign)
    Number,
    /// input a phone number
    Phone,
    Url,
    /// input an URL
    Email,
    /// input an email address
    Name,
    /// input a name of a person
    Password,
    /// input a password
    Pin,
    Terminal,
}
