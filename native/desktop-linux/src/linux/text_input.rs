use std::ffi::CString;

use log::{debug, warn};
use smithay_client_toolkit::reexports::{
    client::{delegate_noop, Connection, Dispatch, Proxy, QueueHandle},
    protocols::wp::text_input::zv3::client::{
        zwp_text_input_manager_v3::ZwpTextInputManagerV3,
        zwp_text_input_v3::{self, ZwpTextInputV3},
    },
};

use crate::linux::events::{ComposedTextChangedEvent, TextInputEvent};

use super::application_state::ApplicationState;

delegate_noop!(ApplicationState: ignore ZwpTextInputManagerV3);

impl Dispatch<ZwpTextInputV3, i32> for ApplicationState {
    fn event(
        this: &mut Self,
        text_input: &ZwpTextInputV3,
        event: <ZwpTextInputV3 as Proxy>::Event,
        _: &i32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwp_text_input_v3::Event::Enter { surface } => {
                debug!("zwp_text_input_v3::Event::Enter: {}", surface.id());
                text_input.enable();
                text_input.set_surrounding_text(String::new(), 0, 0);
                text_input.set_content_type(zwp_text_input_v3::ContentHint::Multiline, zwp_text_input_v3::ContentPurpose::Normal);
                text_input.set_cursor_rectangle(0, 0, 5, 10);
                text_input.commit();
            }
            zwp_text_input_v3::Event::Leave { surface } => {
                debug!("zwp_text_input_v3::Event::Leave: {}", surface.id());
                text_input.disable();
            }
            zwp_text_input_v3::Event::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => {
                debug!("zwp_text_input_v3::Event::PreeditString: cursor_begin={cursor_begin}, cursor_end={cursor_end}, text={text:?}");
                if let Some(w) = this.get_key_window() {
                    let cstr = text.map(|s| CString::new(s).unwrap());
                    let e = ComposedTextChangedEvent::new(cstr.as_ref(), cursor_begin, cursor_end);
                    (w.event_handler)(&e.into());
                }
            }
            zwp_text_input_v3::Event::CommitString { text } => {
                debug!("zwp_text_input_v3::Event::CommitString: {text:?}");
                if let Some(w) = this.get_key_window() {
                    let cstr = text.map(|s| CString::new(s).unwrap());
                    let e = TextInputEvent::new(cstr.as_ref());
                    (w.event_handler)(&e.into());
                }
            }
            zwp_text_input_v3::Event::DeleteSurroundingText {
                before_length,
                after_length,
            } => {
                debug!("zwp_text_input_v3::Event::DeleteSurroundingText: before_length={before_length}, after_length={after_length}");
            }
            zwp_text_input_v3::Event::Done { serial } => {
                debug!("zwp_text_input_v3::Event::Done: serial={serial}");
            }
            _ => {
                warn!("Unknown event: {event:?}");
            },
        }
    }
}
