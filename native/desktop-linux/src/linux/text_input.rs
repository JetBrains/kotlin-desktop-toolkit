use desktop_common::ffi_utils::BorrowedArray;
use log::{debug, warn};
use smithay_client_toolkit::reexports::{
    client::{Connection, Dispatch, Proxy, QueueHandle, delegate_noop},
    protocols::wp::text_input::zv3::client::{
        zwp_text_input_manager_v3::ZwpTextInputManagerV3,
        zwp_text_input_v3::{self, ZwpTextInputV3},
    },
};

use crate::linux::events::{Event, TextInputAvailabilityEvent, TextInputDeleteSurroundingTextData, TextInputEvent};

use super::{application_state::ApplicationState, events::TextInputPreeditStringData};

delegate_noop!(ApplicationState: ignore ZwpTextInputManagerV3);

#[derive(Default)]
pub struct PendingTextInputEvent {
    pub preedit_string: Option<zwp_text_input_v3::Event>,
    pub commit_string: Option<zwp_text_input_v3::Event>,
    pub delete_surrounding_text: Option<zwp_text_input_v3::Event>,
}

impl Dispatch<ZwpTextInputV3, i32> for ApplicationState {
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
                //                text_input.enable();
                //                text_input.set_surrounding_text(String::new(), 0, 0);
                //                text_input.set_content_type(zwp_text_input_v3::ContentHint::Multiline, zwp_text_input_v3::ContentPurpose::Normal);
                //                text_input.set_cursor_rectangle(0, 0, 5, 10);
                //                text_input.commit();
                if let Some(w) = this.get_key_window() {
                    (w.event_handler)(&Event::TextInputAvailability(TextInputAvailabilityEvent { available: true }));
                }
            }
            zwp_text_input_v3::Event::Leave { surface } => {
                debug!("zwp_text_input_v3::Event::Leave: {}", surface.id());
                this.active_text_input = None;
                if let Some(w) = this.get_key_window() {
                    (w.event_handler)(&Event::TextInputAvailability(TextInputAvailabilityEvent { available: false }));
                }
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
                let v = std::mem::take(&mut this.pending_text_input_event);
                if let Some(w) = this.get_key_window() {
                    let (has_commit_string, commit_string) = match v.commit_string {
                        Some(zwp_text_input_v3::Event::CommitString { text }) => (true, text),
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
                        }) => Some((text, cursor_begin, cursor_end)),
                        _ => None,
                    };
                    let e = TextInputEvent {
                        has_preedit_string: preedit_data.is_some(),
                        preedit_string: if let Some((preedit_text, preedit_begin, preedit_end)) = &preedit_data {
                            TextInputPreeditStringData {
                                text_bytes: if let Some(s) = preedit_text {
                                    BorrowedArray::from_slice(s.as_bytes())
                                } else {
                                    BorrowedArray::null()
                                },
                                cursor_begin_byte_pos: *preedit_begin,
                                cursor_end_byte_pos: *preedit_end,
                            }
                        } else {
                            TextInputPreeditStringData::default()
                        },
                        has_commit_string,
                        commit_string: if let Some(s) = &commit_string {
                            BorrowedArray::from_slice(s.as_bytes())
                        } else {
                            BorrowedArray::null()
                        },
                        has_delete_surrounding_text: delete_surrounding_text.is_some(),
                        delete_surrounding_text: delete_surrounding_text.unwrap_or_default(),
                    };
                    (w.event_handler)(&e.into());
                }
            }
            _ => {
                warn!("Unknown event: {event:?}");
            }
        }
    }
}
