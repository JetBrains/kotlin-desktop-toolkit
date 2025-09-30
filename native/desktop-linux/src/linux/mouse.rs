use log::debug;
use smithay_client_toolkit::{
    compositor::SurfaceData,
    delegate_pointer,
    reexports::client::{
        Connection, Dispatch, Proxy as _, QueueHandle,
        protocol::{
            wl_pointer::{AxisRelativeDirection, WlPointer},
            wl_surface::{self, WlSurface},
        },
    },
    seat::pointer::{AxisScroll, PointerData, PointerEvent, PointerEventKind, PointerHandler},
};

use crate::linux::{
    application_state::ApplicationState,
    events::{
        MouseButton, MouseDownEvent, MouseEnteredEvent, MouseExitedEvent, MouseMovedEvent, MouseUpEvent, ScrollData, ScrollWheelEvent,
        Timestamp,
    },
    geometry::LogicalPixels,
};

impl From<AxisScroll> for ScrollData {
    fn from(value: AxisScroll) -> Self {
        let wheel_value120 = if value.value120 != 0 {
            value.value120
        } else {
            value.discrete * 120
        };
        Self {
            delta: LogicalPixels(value.absolute),
            wheel_value120,
            is_inverted: value.relative_direction == Some(AxisRelativeDirection::Inverted),
            is_stop: value.stop,
        }
    }
}

impl PointerHandler for ApplicationState {
    fn pointer_frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, pointer: &WlPointer, events: &[PointerEvent]) {
        for event in events {
            let (window_id, scale) = if let Some(window) = self.windows.get_mut(&event.surface.id()) {
                if let PointerEventKind::Enter { .. } = event.kind {
                    window.set_cursor = true;
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
                    PointerEventKind::Press { button, serial, time } => {
                        if let Some(pointer_data) = pointer.data::<PointerData>() {
                            let seat = pointer_data.seat();
                            self.last_implicit_grab_seat = Some(seat.clone());
                        }
                        self.last_implicit_grab_serial = Some(serial);
                        self.send_event(MouseDownEvent {
                            window_id,
                            button: MouseButton(button),
                            location_in_window: event.position.into(),
                            timestamp: Timestamp(time),
                        })
                    }
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
                    } => {
                        debug!("wl_pointer vertical={vertical:?}");
                        self.send_event(ScrollWheelEvent {
                            window_id,
                            location_in_window: event.position.into(),
                            timestamp: Timestamp(time),
                            horizontal_scroll: horizontal.into(),
                            vertical_scroll: vertical.into(),
                        })
                    }
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
