use crate::gtk::application::send_event;
use crate::gtk::events::{EventHandler, TextInputDeleteSurroundingTextData, TextInputEvent, TextInputPreeditStringData, WindowId};
use crate::gtk::text_input_api::{TextInputContentPurpose, TextInputContextHint, TextInputContextHintBitflag};
use anyhow::anyhow;
use desktop_common::ffi_utils::BorrowedStrPtr;
use enumflags2::BitFlags;
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
            TextInputContentPurpose::Terminal => Self::Terminal,
        }
    }
}

impl TryFrom<TextInputContextHintBitflag> for gtk4::InputHints {
    type Error = anyhow::Error;

    fn try_from(value: TextInputContextHintBitflag) -> Result<Self, Self::Error> {
        let hints = BitFlags::<TextInputContextHint>::from_bits(value.0).map_err(|e| anyhow!(e))?;
        let mut gtk_hints = Self::NONE;
        for hint in hints {
            match hint {
                TextInputContextHint::None => {}
                TextInputContextHint::WordCompletion => gtk_hints.set(Self::WORD_COMPLETION, true),
                TextInputContextHint::Spellcheck => gtk_hints.set(Self::SPELLCHECK, true),
                TextInputContextHint::Lowercase => gtk_hints.set(Self::LOWERCASE, true),
                TextInputContextHint::UppercaseChars => gtk_hints.set(Self::UPPERCASE_CHARS, true),
                TextInputContextHint::UppercaseWords => gtk_hints.set(Self::UPPERCASE_WORDS, true),
                TextInputContextHint::UppercaseSentences => gtk_hints.set(Self::UPPERCASE_SENTENCES, true),
            }
        }
        Ok(gtk_hints)
    }
}

fn get_byte_offset(text: &str, offset: i32) -> i32 {
    let mut it = text.char_indices();
    for _ in 0..offset {
        it.next();
    }
    i32::try_from(it.offset()).unwrap()
}

fn im_commit_handler(event_handler: EventHandler, window_id: WindowId, commit_string: &str) {
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
fn im_preedit_changed_handler(event_handler: EventHandler, window_id: WindowId, im_context: &gtk4::IMMulticontext) {
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

pub fn create_im_context(window_id: WindowId, event_handler: EventHandler) -> gtk4::IMMulticontext {
    let im_context = gtk4::IMMulticontext::new();
    im_context.connect_commit(move |_im_context, commit_string| {
        im_commit_handler(event_handler, window_id, commit_string);
    });

    im_context.connect_preedit_changed(move |im_context| {
        im_preedit_changed_handler(event_handler, window_id, im_context);
    });

    im_context
}
