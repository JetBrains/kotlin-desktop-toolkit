use std::ffi::CString;

use desktop_common::ffi_utils::BorrowedStrPtr;
use log::{debug, warn};
use smithay_client_toolkit::reexports::{
    client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop},
    protocols::wp::text_input::zv3::client::{
        zwp_text_input_manager_v3::ZwpTextInputManagerV3,
        zwp_text_input_v3::{self, ZwpTextInputV3},
    },
};

use crate::linux::{
    application_state::ApplicationState,
    events::{Event, TextInputAvailabilityEvent, TextInputDeleteSurroundingTextData, TextInputEvent, TextInputPreeditStringData},
};

delegate_noop!(ApplicationState: ignore ZwpTextInputManagerV3);

#[derive(Default)]
pub struct PendingTextInputEvent {
    pub preedit_string: Option<zwp_text_input_v3::Event>,
    pub commit_string: Option<zwp_text_input_v3::Event>,
    pub delete_surrounding_text: Option<zwp_text_input_v3::Event>,
}

impl Dispatch<ZwpTextInputV3, i32> for ApplicationState {
    #[allow(clippy::too_many_lines)]
    fn event(
        this: &mut Self,
        text_input: &ZwpTextInputV3,
        event: <ZwpTextInputV3 as Proxy>::Event,
        _: &i32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match &event {
            zwp_text_input_v3::Event::Enter { surface } => {
                debug!("zwp_text_input_v3::Event::Enter: {}", surface.id());
                this.active_text_input = Some(text_input.clone());
                this.active_text_input_surface = Some(surface.id());
                let Some(w) = this.windows.get(&surface.id()) else {
                    warn!("Couldn't find window for: {event:?}");
                    return;
                };
                (w.event_handler)(&Event::TextInputAvailability(TextInputAvailabilityEvent { available: true }));
            }
            zwp_text_input_v3::Event::Leave { surface } => {
                debug!("zwp_text_input_v3::Event::Leave: {}", surface.id());
                let Some(w) = this.windows.get(&surface.id()) else {
                    warn!("Couldn't find window for: {event:?}");
                    return;
                };
                (w.event_handler)(&Event::TextInputAvailability(TextInputAvailabilityEvent { available: false }));
                this.active_text_input = None;
                this.active_text_input_surface = None;
            }
            zwp_text_input_v3::Event::PreeditString {
                text,
                cursor_begin,
                cursor_end,
            } => {
                debug!("zwp_text_input_v3::Event::PreeditString: cursor_begin={cursor_begin}, cursor_end={cursor_end}, text={text:?}");
                this.pending_text_input_event.preedit_string = Some(event);
            }
            zwp_text_input_v3::Event::CommitString { text } => {
                debug!("zwp_text_input_v3::Event::CommitString: {text:?}");
                this.pending_text_input_event.commit_string = Some(event);
            }
            zwp_text_input_v3::Event::DeleteSurroundingText {
                before_length,
                after_length,
            } => {
                debug!("zwp_text_input_v3::Event::DeleteSurroundingText: before_length={before_length}, after_length={after_length}");
                this.pending_text_input_event.delete_surrounding_text = Some(event);
            }
            zwp_text_input_v3::Event::Done { serial } => {
                debug!("zwp_text_input_v3::Event::Done: serial={serial}");
                if this.pending_text_input_event.commit_string.is_none()
                    && this.pending_text_input_event.delete_surrounding_text.is_none()
                    && this.pending_text_input_event.preedit_string.is_none()
                {
                    return;
                }

                let v = std::mem::take(&mut this.pending_text_input_event);
                let Some(active_text_input_surface) = &this.active_text_input_surface else {
                    warn!("No active text input surface for: {event:?}");
                    return;
                };

                let Some(w) = this.windows.get(active_text_input_surface) else {
                    warn!("Couldn't find key window for: {event:?}");
                    return;
                };

                let (has_commit_string, commit_string) = match v.commit_string {
                    Some(zwp_text_input_v3::Event::CommitString { text }) => (true, text.map(|t| CString::new(t).unwrap())),
                    _ => (false, None),
                };
                let delete_surrounding_text = match v.delete_surrounding_text {
                    Some(zwp_text_input_v3::Event::DeleteSurroundingText {
                        before_length,
                        after_length,
                    }) => Some(TextInputDeleteSurroundingTextData {
                        before_length_in_bytes: before_length,
                        after_length_in_bytes: after_length,
                    }),
                    _ => None,
                };
                let preedit_data = match v.preedit_string {
                    Some(zwp_text_input_v3::Event::PreeditString {
                        text,
                        cursor_begin,
                        cursor_end,
                    }) => Some((text.map(|t| CString::new(t).unwrap()), cursor_begin, cursor_end)),
                    _ => None,
                };
                let e = TextInputEvent {
                    has_preedit_string: preedit_data.is_some(),
                    preedit_string: if let Some((preedit_text, preedit_begin, preedit_end)) = &preedit_data {
                        TextInputPreeditStringData {
                            text: BorrowedStrPtr::new_optional(preedit_text.as_ref()),
                            cursor_begin_byte_pos: *preedit_begin,
                            cursor_end_byte_pos: *preedit_end,
                        }
                    } else {
                        TextInputPreeditStringData::default()
                    },
                    has_commit_string,
                    commit_string: BorrowedStrPtr::new_optional(commit_string.as_ref()),
                    has_delete_surrounding_text: delete_surrounding_text.is_some(),
                    delete_surrounding_text: delete_surrounding_text.unwrap_or_default(),
                };
                (w.event_handler)(&e.into());
            }
            _ => {
                warn!("Unknown event: {event:?}");
            }
        }
    }
}
