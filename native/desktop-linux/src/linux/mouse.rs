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
    events::{
        MouseButton, MouseDownEvent, MouseEnteredEvent, MouseExitedEvent, MouseMovedEvent, MouseUpEvent, ScrollWheelEvent, Timestamp,
    },
    geometry::LogicalPixels,
};

impl PointerHandler for ApplicationState {
    fn pointer_frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, pointer: &WlPointer, events: &[PointerEvent]) {
        for event in events {
            let (window_id, scale) = if let Some(window) = self.windows.get_mut(&event.surface.id()) {
                match event.kind {
                    PointerEventKind::Enter { .. } => {
                        window.set_cursor = true;
                    }
                    PointerEventKind::Press {
                        button: _,
                        serial,
                        time: _,
                    } => {
                        let pointer_data = pointer.data::<PointerData>().unwrap();
                        let seat = pointer_data.seat();
                        window.current_mouse_down_seat = Some(seat.clone());
                        window.current_mouse_down_serial = Some(serial);
                    }
                    // PointerEventKind::Release { button: _, serial: _, time: _ } => {
                    //     self.current_mouse_down_seat = None;
                    //     self.current_mouse_down_serial = None;
                    // }
                    _ => {}
                }
                let scale = window.current_scale;
                (Some(window.window_id), scale)
            } else {
                (None, 1.0)
            };

            if let Some(window_id) = window_id {
                _ = match event.kind {
                    PointerEventKind::Enter { .. } => self.send_event(MouseEnteredEvent {
                        window_id,
                        location_in_window: event.position.into(),
                    }),
                    PointerEventKind::Leave { .. } => self.send_event(MouseExitedEvent {
                        window_id,
                        location_in_window: event.position.into(),
                    }),
                    PointerEventKind::Motion { time } => self.send_event(MouseMovedEvent {
                        window_id,
                        location_in_window: event.position.into(),
                        timestamp: Timestamp(time),
                    }),
                    PointerEventKind::Press { button, serial: _, time } => self.send_event(MouseDownEvent {
                        window_id,
                        button: MouseButton(button),
                        location_in_window: event.position.into(),
                        timestamp: Timestamp(time),
                    }),
                    PointerEventKind::Release { button, serial: _, time } => self.send_event(MouseUpEvent {
                        window_id,
                        button: MouseButton(button),
                        location_in_window: event.position.into(),
                        timestamp: Timestamp(time),
                    }),
                    PointerEventKind::Axis {
                        time,
                        horizontal,
                        vertical,
                        ..
                    } => self.send_event(ScrollWheelEvent {
                        window_id,
                        scrolling_delta_x: LogicalPixels(horizontal.absolute),
                        scrolling_delta_y: LogicalPixels(vertical.absolute),
                        location_in_window: event.position.into(),
                        timestamp: Timestamp(time),
                    }),
                }
            }

            if let PointerEventKind::Enter { .. } = event.kind
                && let Some(themed_pointer) = self.themed_pointer.take()
            {
                let pointer_surface = themed_pointer.surface();
                if let Some(pointer_surface_data) = pointer_surface.data() {
                    #[allow(clippy::cast_possible_truncation)]
                    let pointer_surface_event = wl_surface::Event::PreferredBufferScale {
                        factor: scale.round() as i32,
                    };
                    debug!("Setting cursor scale to {scale:?}");
                    Dispatch::<WlSurface, SurfaceData>::event(self, pointer_surface, pointer_surface_event, pointer_surface_data, conn, qh);
                }
                self.themed_pointer = Some(themed_pointer);
            }
        }
    }
}

delegate_pointer!(ApplicationState);
