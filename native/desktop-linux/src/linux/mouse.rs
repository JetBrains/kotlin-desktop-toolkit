use std::time::Duration;

use log::debug;
use smithay_client_toolkit::reexports::client::{Proxy, protocol::wl_pointer};
use smithay_client_toolkit::reexports::csd_frame::{DecorationsFrame, FrameClick};
use smithay_client_toolkit::{
    seat::pointer::{PointerEvent, PointerEventKind},
    shell::WaylandSurface,
};

use super::window::SimpleWindow;

impl SimpleWindow {
    pub fn pointer_frame(&mut self, pointer: &wl_pointer::WlPointer, event: &PointerEvent) {
        use PointerEventKind::{Axis, Enter, Leave, Motion, Press, Release};
        let (x, y) = event.position;
        match event.kind {
            Enter { .. } => {
                self.set_cursor = true;
                self.decorations_cursor = self
                    .window_frame
                    .as_mut()
                    .and_then(|frame| frame.click_point_moved(Duration::ZERO, &event.surface.id(), x, y));
            }
            Leave { .. } => {
                if &event.surface != self.window.wl_surface() {
                    if let Some(window_frame) = self.window_frame.as_mut() {
                        window_frame.click_point_left();
                    }
                }
            }
            Motion { time } => {
                if let Some(new_cursor) = self
                    .window_frame
                    .as_mut()
                    .and_then(|frame| frame.click_point_moved(Duration::from_millis(u64::from(time)), &event.surface.id(), x, y))
                {
                    self.set_cursor = true;
                    self.decorations_cursor = Some(new_cursor);
                }
            }
            Press { button, serial, time } | Release { button, serial, time } => {
                let pressed = matches!(event.kind, Press { .. });
                if &event.surface != self.window.wl_surface() {
                    let click = match button {
                        0x110 => FrameClick::Normal,
                        0x111 => FrameClick::Alternate,
                        _ => return,
                    };

                    if let Some(action) = self
                        .window_frame
                        .as_mut()
                        .and_then(|frame| frame.on_click(Duration::from_millis(u64::from(time)), click, pressed))
                    {
                        debug!("Frame click action {action:?}");
                        self.frame_action(pointer, serial, action);
                    }
                } else if pressed {
                    self.shift = self.shift.xor(Some(0));
                }
            }
            Axis { .. } => {}
        }
    }
}
