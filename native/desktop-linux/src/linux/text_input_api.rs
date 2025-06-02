use desktop_common::ffi_utils::BorrowedStrPtr;
use log::debug;
use smithay_client_toolkit::reexports::protocols::wp::text_input::zv3::client::zwp_text_input_v3;

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

impl TextInputContext<'_> {
    fn get_byte_offset(text: &str, offset: u16) -> usize {
        let mut it = text.char_indices();
        for _ in 0..offset {
            it.next();
        }
        it.offset()
    }

    pub(crate) fn apply(&self, text_input: &zwp_text_input_v3::ZwpTextInputV3) -> anyhow::Result<()> {
        let surrounding_text = self.surrounding_text.as_str()?;
        let content_hint = if self.is_multiline {
            zwp_text_input_v3::ContentHint::Multiline
        } else {
            zwp_text_input_v3::ContentHint::None
        };

        let cursor_pos_bytes = Self::get_byte_offset(surrounding_text, self.cursor_codepoint_offset);

        let selection_start_pos_bytes = if self.selection_start_codepoint_offset == self.cursor_codepoint_offset {
            cursor_pos_bytes
        } else {
            Self::get_byte_offset(surrounding_text, self.selection_start_codepoint_offset)
        };

        debug!(
            "Calling set_surrounding_text with cursor_pos_bytes={cursor_pos_bytes}, selection_start_pos_bytes={selection_start_pos_bytes}, surrounding_text={surrounding_text}"
        );
        #[allow(clippy::cast_possible_truncation)]
        text_input.set_surrounding_text(
            surrounding_text.to_owned(),
            cursor_pos_bytes as i32,
            selection_start_pos_bytes as i32,
        );
        text_input.set_content_type(content_hint, self.content_purpose.to_system());
        text_input.set_text_change_cause(if self.change_caused_by_input_method {
            zwp_text_input_v3::ChangeCause::InputMethod
        } else {
            zwp_text_input_v3::ChangeCause::Other
        });
        text_input.set_cursor_rectangle(
            self.cursor_rectangle.origin.x.round(),
            self.cursor_rectangle.origin.y.round(),
            self.cursor_rectangle.size.width.round(),
            self.cursor_rectangle.size.height.round(),
        );
        text_input.commit();
        Ok(())
    }
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

impl TextInputContentPurpose {
    const fn to_system(&self) -> zwp_text_input_v3::ContentPurpose {
        match self {
            Self::Normal => zwp_text_input_v3::ContentPurpose::Normal,
            Self::Alpha => zwp_text_input_v3::ContentPurpose::Alpha,
            Self::Digits => zwp_text_input_v3::ContentPurpose::Digits,
            Self::Number => zwp_text_input_v3::ContentPurpose::Number,
            Self::Phone => zwp_text_input_v3::ContentPurpose::Phone,
            Self::Url => zwp_text_input_v3::ContentPurpose::Url,
            Self::Email => zwp_text_input_v3::ContentPurpose::Email,
            Self::Name => zwp_text_input_v3::ContentPurpose::Name,
            Self::Password => zwp_text_input_v3::ContentPurpose::Password,
            Self::Pin => zwp_text_input_v3::ContentPurpose::Pin,
            Self::Date => zwp_text_input_v3::ContentPurpose::Date,
            Self::Time => zwp_text_input_v3::ContentPurpose::Time,
            Self::Datetime => zwp_text_input_v3::ContentPurpose::Datetime,
            Self::Terminal => zwp_text_input_v3::ContentPurpose::Terminal,
        }
    }
}
