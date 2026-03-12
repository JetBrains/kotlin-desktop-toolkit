use crate::gtk::geometry::LogicalRect;
use enumflags2::{BitFlags, bitflags};

#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextInputContextHint {
    Spellcheck = 1 << 0,
    NoSpellcheck = 1 << 1,
    WordCompletion = 1 << 2,
    Lowercase = 1 << 3,
    UppercaseChars = 1 << 4,
    UppercaseWords = 1 << 5,
    UppercaseSentences = 1 << 6,
    InhibitOsk = 1 << 7,
    VerticalWriting = 1 << 8,
    Emoji = 1 << 9,
    NoEmoji = 1 << 10,
    Private = 1 << 11,
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
pub struct TextInputContext {
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
