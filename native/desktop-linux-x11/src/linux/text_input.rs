use crate::linux::application::send_event;
use crate::linux::events::{EventHandler, TextInputDeleteSurroundingTextData, TextInputEvent, TextInputPreeditStringData, WindowId};
use crate::linux::text_input_api::TextInputContentPurpose;
use desktop_common::ffi_utils::BorrowedStrPtr;
use gtk4::prelude::IMContextExt;
use log::debug;
use std::ffi::CString;

impl From<TextInputContentPurpose> for gtk4::InputPurpose {
    fn from(value: TextInputContentPurpose) -> Self {
        match value {
            TextInputContentPurpose::Normal => Self::FreeForm,
            TextInputContentPurpose::Alpha => Self::Alpha,
            TextInputContentPurpose::Digits => Self::Digits,
            TextInputContentPurpose::Number => Self::Number,
            TextInputContentPurpose::Phone => Self::Phone,
            TextInputContentPurpose::Url => Self::Url,
            TextInputContentPurpose::Email => Self::Email,
            TextInputContentPurpose::Name => Self::Name,
            TextInputContentPurpose::Password => Self::Password,
            TextInputContentPurpose::Pin => Self::Pin,
            TextInputContentPurpose::Date => Self::FreeForm,     // not exposed in GTK
            TextInputContentPurpose::Time => Self::FreeForm,     // not exposed in GTK
            TextInputContentPurpose::Datetime => Self::FreeForm, // not exposed in GTK
            TextInputContentPurpose::Terminal => Self::Terminal,
        }
    }
}

fn get_byte_offset(text: &str, offset: i32) -> i32 {
    let mut it = text.char_indices();
    for _ in 0..offset {
        it.next();
    }
    i32::try_from(it.offset()).unwrap()
}

pub fn im_commit_handler(event_handler: EventHandler, window_id: WindowId, commit_string: &str) {
    debug!("commit for {window_id:?}: commit_string={commit_string}");
    let commit_string_cstr = CString::new(commit_string).unwrap();
    let event = TextInputEvent {
        window_id,
        has_preedit_string: false,
        preedit_string: TextInputPreeditStringData::default(),
        has_commit_string: true,
        commit_string: BorrowedStrPtr::new(&commit_string_cstr),
        has_delete_surrounding_text: false,
        delete_surrounding_text: TextInputDeleteSurroundingTextData::default(),
    };
    send_event(event_handler, event);
}

#[allow(clippy::similar_names)]
pub fn im_preedit_changed_handler(event_handler: EventHandler, window_id: WindowId, im_context: &gtk4::IMMulticontext) {
    let (preedit_gstring, _attrs, cursor_char_pos) = im_context.preedit_string();
    debug!("preedit_changed for {window_id:?}: str={preedit_gstring}, cursor_pos={cursor_char_pos}");
    let text_string = preedit_gstring.as_str();

    let cursor_begin_byte_pos = get_byte_offset(text_string, cursor_char_pos);
    let cursor_end_byte_pos = get_byte_offset(text_string, cursor_char_pos);
    let text_cstr = CString::new(preedit_gstring.as_str()).unwrap();

    let preedit_string = TextInputPreeditStringData {
        text: BorrowedStrPtr::new(&text_cstr),
        cursor_begin_byte_pos,
        cursor_end_byte_pos,
    };
    let event = TextInputEvent {
        window_id,
        has_preedit_string: true,
        preedit_string,
        has_commit_string: false,
        commit_string: BorrowedStrPtr::null(),
        has_delete_surrounding_text: false,
        delete_surrounding_text: TextInputDeleteSurroundingTextData::default(),
    };
    send_event(event_handler, event);
}
