use objc2_app_kit::NSEvent;

use crate::{common::Point, macos::window};
use anyhow::{anyhow, Result};

use super::{application_api::AppState, window::WindowId};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    window_id: WindowId,
    point: Point
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    window_id: WindowId,
    dx: f64,
    dy: f64
}

#[repr(C)]
#[derive(Debug)]
pub enum Event {
    MouseMoved(MouseMovedEvent),
    ScrollWheel(ScrollWheelEvent)
}

pub(crate) fn handle_mouse_moved(event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        println!("event: {event:?}");
        let point = unsafe {
            event.locationInWindow()
        };
        let window_id = unsafe {
            event.windowNumber() as i64
        };
        let window = unsafe {
            event.window(state.mtm).expect(&format!("No window for event: {event:?}"))
        };
        let frame = window.frame();

        let event = Event::MouseMoved(MouseMovedEvent {
            window_id,
            point: Point {
                x: point.x,
                y: frame.size.height - point.y,
            },
        });
        (state.event_handler)(&event)
    });
    handled
}