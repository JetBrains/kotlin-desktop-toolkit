use crate::gtk::application::send_event;
use crate::gtk::application_api::FfiTextInputSurroundingText;
use crate::gtk::events::{
    EventHandler, TextInputDeleteSurroundingTextData, TextInputEvent, TextInputPreeditAttribute, TextInputPreeditStringData,
    TextInputPreeditUnderlineType, WindowId,
};
use crate::gtk::ffi_return_conversions::RetrieveSurroundingText;
use crate::gtk::text_input_api::{TextInputContentPurpose, TextInputContextHints};
use anyhow::bail;
use desktop_common::ffi_utils::{BorrowedArray, BorrowedStrPtr};
use gtk4::glib::translate::FromGlib;
use gtk4::pango;
use gtk4::prelude::IMContextExt;
use log::debug;
use std::cmp::Ordering;
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

impl TryFrom<TextInputContextHints> for gtk4::InputHints {
    type Error = anyhow::Error;

    fn try_from(value: TextInputContextHints) -> Result<Self, Self::Error> {
        let mut gtk_hints = Self::NONE;
        for hint in value {
            match hint {
                TextInputContextHints::Spellcheck => gtk_hints.set(Self::SPELLCHECK, true),
                TextInputContextHints::NoSpellcheck => gtk_hints.set(Self::NO_SPELLCHECK, true),
                TextInputContextHints::WordCompletion => gtk_hints.set(Self::WORD_COMPLETION, true),
                TextInputContextHints::Lowercase => gtk_hints.set(Self::LOWERCASE, true),
                TextInputContextHints::UppercaseChars => gtk_hints.set(Self::UPPERCASE_CHARS, true),
                TextInputContextHints::UppercaseWords => gtk_hints.set(Self::UPPERCASE_WORDS, true),
                TextInputContextHints::UppercaseSentences => gtk_hints.set(Self::UPPERCASE_SENTENCES, true),
                TextInputContextHints::InhibitOsk => gtk_hints.set(Self::INHIBIT_OSK, true),
                TextInputContextHints::VerticalWriting => gtk_hints.set(Self::VERTICAL_WRITING, true),
                TextInputContextHints::Emoji => gtk_hints.set(Self::EMOJI, true),
                TextInputContextHints::NoEmoji => gtk_hints.set(Self::NO_EMOJI, true),
                TextInputContextHints::Private => gtk_hints.set(Self::PRIVATE, true),
                _ => bail!("Unknown hint: {hint:?}"),
            }
        }
        Ok(gtk_hints)
    }
}

#[derive(Debug)]
pub struct SurroundingTextWithSelection<'a> {
    text: &'a str,
    cursor_char_pos: u16,
    cursor_byte_index: i32,
    anchor_byte_index: i32,
}

impl<'a> SurroundingTextWithSelection<'a> {
    pub fn new(context: &'a FfiTextInputSurroundingText) -> anyhow::Result<Self> {
        let text = str::from_utf8(context.surrounding_text.as_slice()?)?;
        let cursor_char_pos = context.cursor_codepoint_offset;
        let anchor_char_pos = context.selection_start_codepoint_offset;
        let (cursor_byte_index, anchor_byte_index) = match cursor_char_pos.cmp(&anchor_char_pos) {
            Ordering::Equal => {
                let (cursor_byte_index, _) = get_byte_offset(text, cursor_char_pos.into());
                (cursor_byte_index, cursor_byte_index)
            }
            Ordering::Less => {
                let (cursor_byte_index, text_from_cursor) = get_byte_offset(text, cursor_char_pos.into());
                let (anchor_bytes_diff, _) = get_byte_offset(text_from_cursor, (anchor_char_pos - cursor_char_pos).into());
                (cursor_byte_index, cursor_byte_index + anchor_bytes_diff)
            }
            Ordering::Greater => {
                let (anchor_byte_index, text_from_anchor) = get_byte_offset(text, anchor_char_pos.into());
                let (cursor_bytes_diff, _) = get_byte_offset(text_from_anchor, (cursor_char_pos - anchor_char_pos).into());
                (anchor_byte_index + cursor_bytes_diff, anchor_byte_index)
            }
        };
        Ok(Self {
            text,
            cursor_char_pos,
            cursor_byte_index,
            anchor_byte_index,
        })
    }
}

fn get_byte_offset(text: &str, offset: i32) -> (i32, &str) {
    let mut it = text.char_indices();
    for _ in 0..offset {
        it.next();
    }
    (i32::try_from(it.offset()).unwrap(), it.as_str())
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

impl TextInputPreeditAttribute {
    const fn default(begin_byte_pos: u32, end_byte_pos: u32) -> Self {
        Self {
            begin_byte_pos,
            end_byte_pos,
            underline: TextInputPreeditUnderlineType::None,
            foreground_highlight: false,
            background_highlight: false,
            strikethrough: false,
            bold: false,
            italic: false,
        }
    }

    fn apply_from_pango(&mut self, pango_attr: pango::Attribute) {
        // Handle attributes used by popular IMEs, see
        // https://github.com/ibus/ibus/blob/a5624b25b8ae666eaffb5b1d0fb197358ce4e9ef/client/gtk2/ibusimcontext.c#L2255
        // https://github.com/ibus/ibus/blob/a5624b25b8ae666eaffb5b1d0fb197358ce4e9ef/src/ibusattribute.h#L65
        // https://github.com/fcitx/fcitx5-gtk/blob/049e622f774f5bc34cef8b1e5151b9cc4b9601f6/gtk4/fcitximcontext.cpp#L551
        // https://github.com/fcitx/fcitx5/blob/c2c757f0e3d434fba6c82a509e0896679ed6dede/src/lib/fcitx-utils/textformatflags.h#L20
        // https://github.com/uim/uim/blob/8227191161636b12017026db17230045033a6d0b/gtk4/immodule/uim-im-context.c#L691
        // https://github.com/uim/uim/blob/8227191161636b12017026db17230045033a6d0b/uim/uim.h#L439

        match pango_attr.attr_class().type_() {
            pango::AttrType::Underline => {
                let pango_underline_attr = pango_attr.downcast::<pango::AttrInt>().unwrap();
                let pango_underline = unsafe { pango::Underline::from_glib(pango_underline_attr.value()) };
                debug!("preedit Underline attribute: {pango_underline:?}");
                let underline = match pango_underline {
                    pango::Underline::Single => TextInputPreeditUnderlineType::Single,
                    pango::Underline::Double => TextInputPreeditUnderlineType::Double,
                    pango::Underline::Low => TextInputPreeditUnderlineType::Low,
                    pango::Underline::Error => TextInputPreeditUnderlineType::Error,
                    _ => TextInputPreeditUnderlineType::None,
                };
                self.underline = underline;
            }
            pango::AttrType::Foreground => {
                debug!("preedit Foreground attribute");
                self.foreground_highlight = true;
            }
            pango::AttrType::Background => {
                debug!("preedit Background attribute");
                self.background_highlight = true;
            }
            pango::AttrType::Strikethrough => {
                debug!("preedit Strikethrough attribute");
                self.strikethrough = true;
            }
            pango::AttrType::Weight => {
                let pango_weight_attr = pango_attr.downcast::<pango::AttrInt>().unwrap();
                let weight = unsafe { pango::Weight::from_glib(pango_weight_attr.value()) };
                debug!("preedit Weight attribute: {weight:?}");
                if weight == pango::Weight::Bold {
                    self.bold = true;
                }
            }
            pango::AttrType::Style => {
                let pango_style_attr = pango_attr.downcast::<pango::AttrInt>().unwrap();
                let style = unsafe { pango::Style::from_glib(pango_style_attr.value()) };
                debug!("preedit Style attribute: {style:?}");
                if style == pango::Style::Italic {
                    self.italic = true;
                }
            }
            _ => {}
        }
    }
}

fn mut_find_or_insert<T>(vec: &mut Vec<T>, predicate: impl Fn(&T) -> bool, insert: impl FnOnce() -> T) -> &mut T {
    if let Some(i) = vec.iter().position(predicate) {
        &mut vec[i]
    } else {
        vec.push(insert());
        vec.last_mut().unwrap()
    }
}

#[allow(clippy::similar_names)]
fn im_preedit_changed_handler(event_handler: EventHandler, window_id: WindowId, im_context: &gtk4::IMMulticontext) {
    let (preedit_gstring, pango_attrs, cursor_char_pos) = im_context.preedit_string();
    debug!("preedit_changed for {window_id:?}: str={preedit_gstring}, cursor_pos={cursor_char_pos}");

    let mut attributes = Vec::new();
    for pango_attr in pango_attrs.attributes() {
        let begin_byte_pos = pango_attr.start_index();
        let end_byte_pos = pango_attr.end_index();
        let attr = mut_find_or_insert(
            &mut attributes,
            |e: &TextInputPreeditAttribute| e.begin_byte_pos == begin_byte_pos && e.end_byte_pos == end_byte_pos,
            || TextInputPreeditAttribute::default(begin_byte_pos, end_byte_pos),
        );

        attr.apply_from_pango(pango_attr);
    }

    let text_string = preedit_gstring.as_str();
    let (cursor_byte_pos, _) = get_byte_offset(text_string, cursor_char_pos);
    let text_cstr = CString::new(preedit_gstring.as_str()).unwrap();

    let preedit_string = TextInputPreeditStringData {
        text: BorrowedStrPtr::new(&text_cstr),
        cursor_byte_pos,
        attributes: BorrowedArray::from_slice(&attributes),
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

fn create_text_input_delete_surrounding_text_data(
    window_id: WindowId,
    offset_in_chars: i32,
    n_chars: i32,
    surrounding: &SurroundingTextWithSelection,
) -> TextInputDeleteSurroundingTextData {
    debug!("delete_surrounding for {window_id:?}:, offset_in_chars={offset_in_chars}, n_chars={n_chars}, surrounding={surrounding:?}");
    let text_string = &surrounding.text;
    assert!(offset_in_chars <= 0);
    let (offset_byte_pos, string_from_offset) = get_byte_offset(text_string, i32::from(surrounding.cursor_char_pos) + offset_in_chars);
    let (end_bytes_diff, _) = get_byte_offset(string_from_offset, n_chars);
    let end_byte_pos = offset_byte_pos + end_bytes_diff;

    let before_length_in_bytes = u32::try_from(surrounding.cursor_byte_index - offset_byte_pos).unwrap();
    let after_length_in_bytes = u32::try_from(end_byte_pos - surrounding.cursor_byte_index).unwrap();

    TextInputDeleteSurroundingTextData {
        before_length_in_bytes,
        after_length_in_bytes,
    }
}

pub fn create_im_context(
    window_id: WindowId,
    event_handler: EventHandler,
    retrieve_surrounding_text: RetrieveSurroundingText,
) -> gtk4::IMMulticontext {
    let im_context = gtk4::IMMulticontext::new();
    im_context.connect_commit(move |_im_context, commit_string| {
        im_commit_handler(event_handler, window_id, commit_string);
    });

    im_context.connect_preedit_changed(move |im_context| {
        im_preedit_changed_handler(event_handler, window_id, im_context);
    });

    im_context.connect_preedit_start(move |_im_context| {
        debug!("preedit_start");
    });
    im_context.connect_preedit_end(move |_im_context| {
        debug!("preedit_end");

        let event = TextInputEvent {
            window_id,
            has_preedit_string: false,
            preedit_string: TextInputPreeditStringData::default(),
            has_commit_string: false,
            commit_string: BorrowedStrPtr::null(),
            has_delete_surrounding_text: false,
            delete_surrounding_text: TextInputDeleteSurroundingTextData::default(),
        };
        send_event(event_handler, event);
    });

    im_context.connect_delete_surrounding(move |_im_context, offset, n_chars| {
        retrieve_surrounding_text.with(window_id, move |surrounding| {
            if let Ok(surrounding) = surrounding {
                let delete_surrounding_text = create_text_input_delete_surrounding_text_data(window_id, offset, n_chars, &surrounding);
                let event = TextInputEvent {
                    window_id,
                    has_preedit_string: false,
                    preedit_string: TextInputPreeditStringData::default(),
                    has_commit_string: false,
                    commit_string: BorrowedStrPtr::null(),
                    has_delete_surrounding_text: true,
                    delete_surrounding_text,
                };
                send_event(event_handler, event);
                true
            } else {
                false
            }
        })
    });

    im_context.connect_retrieve_surrounding(move |im_context| {
        retrieve_surrounding_text.with(window_id, move |surrounding| {
            if let Ok(surrounding) = surrounding {
                debug!("IMContext: retrieve_surrounding: {surrounding:?}");
                im_context.set_surrounding_with_selection(surrounding.text, surrounding.cursor_byte_index, surrounding.anchor_byte_index);
                true
            } else {
                debug!("IMContext: retrieve_surrounding: returning false");
                false
            }
        })
    });

    im_context
}
