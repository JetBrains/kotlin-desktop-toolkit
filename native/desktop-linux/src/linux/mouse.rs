use smithay_client_toolkit::reexports::client::Proxy;
use smithay_client_toolkit::reexports::client::protocol::wl_pointer;
use smithay_client_toolkit::seat::pointer::PointerData;
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind};

use crate::linux::events::Event;

use super::events::MouseDownEvent;
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
                let pointer_data = pointer.data::<PointerData>().unwrap();
                let seat = pointer_data.seat();
                self.current_mouse_down_seat = Some(seat.clone());
                self.current_mouse_down_serial = Some(serial);
                (self.event_handler)(&(&e).into());
            }
            PointerEventKind::Release { button, serial: _, time } => {
                //self.current_mouse_down_seat = None;
                //self.current_mouse_down_serial = None;
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
