use log::debug;
use smithay_client_toolkit::{
    compositor::SurfaceData,
    delegate_pointer,
    reexports::client::{
        Connection, Dispatch, Proxy, QueueHandle,
        protocol::{wl_pointer::WlPointer, wl_surface, wl_surface::WlSurface},
    },
    seat::pointer::{PointerData, PointerEvent, PointerEventKind, PointerHandler},
};

use crate::linux::{
    application_state::ApplicationState,
    events::{Event, MouseDownEvent, MouseEnteredEvent, MouseExitedEvent, MouseMovedEvent, MouseUpEvent, ScrollWheelEvent},
    window::SimpleWindow,
};

impl PointerHandler for ApplicationState {
    fn pointer_frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, pointer: &WlPointer, events: &[PointerEvent]) {
        for event in events {
            #[allow(clippy::cast_possible_truncation)]
            let (event_to_send, scale) = if let Some(window) = self.windows.get_mut(&event.surface.id()) {
                let pointer_event = window.make_pointer_event(pointer, event);
                let scale = window.current_scale.round() as i32;
                (Some(pointer_event), scale)
            } else {
                (None, 1)
            };

            if let Some(event_to_send) = event_to_send {
                self.send_event(event_to_send);
            }

            if let PointerEventKind::Enter { .. } = event.kind {
                if let Some(themed_pointer) = self.themed_pointer.take() {
                    let pointer_surface = themed_pointer.surface();
                    if let Some(pointer_surface_data) = pointer_surface.data() {
                        let pointer_surface_event = wl_surface::Event::PreferredBufferScale { factor: scale };
                        debug!("Setting cursor scale to {scale:?}");
                        Dispatch::<WlSurface, SurfaceData>::event(
                            self,
                            pointer_surface,
                            pointer_surface_event,
                            pointer_surface_data,
                            conn,
                            qh,
                        );
                    }
                    self.themed_pointer = Some(themed_pointer);
                }
            }
        }
    }
}

delegate_pointer!(ApplicationState);

impl SimpleWindow {
    pub fn make_pointer_event(&mut self, pointer: &WlPointer, event: &PointerEvent) -> Event<'static> {
        match event.kind {
            PointerEventKind::Enter { .. } => {
                self.set_cursor = true;
                MouseEnteredEvent::new(self.window_id, event).into()
            }
            PointerEventKind::Leave { .. } => MouseExitedEvent::new(self.window_id, event).into(),
            PointerEventKind::Motion { time } => MouseMovedEvent::new(self.window_id, event, time).into(),
            PointerEventKind::Press { button, serial, time } => {
                let e = MouseDownEvent::new(self.window_id, event, button, time);
                let pointer_data = pointer.data::<PointerData>().unwrap();
                let seat = pointer_data.seat();
                self.current_mouse_down_seat = Some(seat.clone());
                self.current_mouse_down_serial = Some(serial);
                e.into()
            }
            PointerEventKind::Release { button, serial: _, time } => {
                //self.current_mouse_down_seat = None;
                //self.current_mouse_down_serial = None;
                MouseUpEvent::new(self.window_id, event, button, time).into()
            }
            PointerEventKind::Axis {
                time,
                horizontal,
                vertical,
                ..
            } => ScrollWheelEvent::new(self.window_id, event, time, horizontal, vertical).into(),
        }
    }
}
