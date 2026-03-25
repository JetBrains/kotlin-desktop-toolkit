use desktop_common::ffi_utils::BorrowedStrPtr;
use enumflags2::{BitFlags, bitflags};

use crate::linux::geometry::LogicalRect;

#[repr(C)]
#[derive(Debug)]
pub struct TextInputContext<'a> {
    pub surrounding_text: BorrowedStrPtr<'a>,
    pub cursor_codepoint_offset: u16,
    pub selection_start_codepoint_offset: u16,
    pub hints: TextInputContentHintBitflag,
    pub content_purpose: TextInputContentPurpose,
    pub cursor_rectangle: LogicalRect,
    pub change_caused_by_input_method: bool,
}

#[repr(C)]
#[derive(Debug)]
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
    /// input a password (combine with `sensitive_data` hint)
    Pin,
    /// input is a numeric password (combine with `sensitive_data` hint)
    Date,
    /// input a date
    Time,
    Datetime,
    Terminal,
}

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextInputContentHint {
    Completion = 1 << 0,
    Spellcheck = 1 << 1,
    AutoCapitalization = 1 << 2,
    Lowercase = 1 << 3,
    Uppercase = 1 << 4,
    Titlecase = 1 << 5,
    HiddenText = 1 << 6,
    SensitiveData = 1 << 7,
    Latin = 1 << 8,
    Multiline = 1 << 9,
}

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct TextInputContentHintBitflag(pub u32);

impl From<TextInputContentHint> for TextInputContentHintBitflag {
    fn from(value: TextInputContentHint) -> Self {
        Self(BitFlags::from_flag(value).bits_c())
    }
}
