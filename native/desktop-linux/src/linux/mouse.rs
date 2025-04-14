use smithay_client_toolkit::reexports::client::Proxy;
use smithay_client_toolkit::reexports::client::protocol::wl_pointer;
use smithay_client_toolkit::seat::pointer::PointerData;
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind};

use crate::linux::events::Event;

use super::events::{MouseDownEvent, WindowFrameAction};
use super::window::SimpleWindow;

impl SimpleWindow {
    pub fn pointer_event(&mut self, pointer: &wl_pointer::WlPointer, event: &PointerEvent) {
        match event.kind {
            PointerEventKind::Enter { .. } => {
                self.set_cursor = true;
                (self.event_handler)(&Event::new_mouse_enter_event(event));
            }
            PointerEventKind::Leave { .. } => {
                (self.event_handler)(&Event::new_mouse_exit_event(event));
            }
            PointerEventKind::Motion { time } => {
                (self.event_handler)(&Event::new_mouse_move_event(event, time));
            }
            PointerEventKind::Press { button, serial, time } => {
                let e = MouseDownEvent::new(event, button, time);
                (self.event_handler)(&(&e).into());
                if e.frame_action_out != WindowFrameAction::None {
                    let pointer_data = pointer.data::<PointerData>().unwrap();
                    let seat = pointer_data.seat();
                    self.frame_action(seat, serial, e.frame_action_out);
                }
            }
            PointerEventKind::Release { button, serial: _, time } => {
                (self.event_handler)(&Event::new_mouse_up_event(event, button, time));
            }
            PointerEventKind::Axis {
                time,
                horizontal,
                vertical,
                ..
            } => {
                (self.event_handler)(&Event::new_scroll_wheel_event(event, time, horizontal, vertical));
            }
        }
    }
}
