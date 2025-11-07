use desktop_common::ffi_utils::BorrowedStrPtr;

use crate::linux::geometry::LogicalRect;

#[repr(C)]
#[derive(Debug)]
pub struct TextInputContext<'a> {
    pub surrounding_text: BorrowedStrPtr<'a>,
    pub cursor_codepoint_offset: u16,
    pub selection_start_codepoint_offset: u16,
    pub is_multiline: bool,
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
