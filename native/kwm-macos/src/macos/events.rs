use objc2_app_kit::{NSEvent, NSWindow};
use objc2_foundation::MainThreadMarker;

use crate::{common::{LogicalPixels, LogicalPoint, LogicalSize}, macos::window};
use anyhow::{anyhow, Result};

use super::{application_api::AppState, screen::{NSScreenExts, ScreenId}, window::{NSWindowExts, WindowId}};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    window_id: WindowId,
    point: LogicalPoint
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDownEvent {
    window_id: WindowId,
    point: LogicalPoint
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseUpEvent {
    window_id: WindowId,
    point: LogicalPoint
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    window_id: WindowId,
    dx: LogicalPixels,
    dy: LogicalPixels
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScreenChangeEvent {
    window_id: WindowId,
    new_screen_id: ScreenId
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowResizeEvent {
    window_id: WindowId,
    size: LogicalSize
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowMoveEvent {
    window_id: WindowId,
    origin: LogicalPoint
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFocusChangeEvent {
    window_id: WindowId,
    is_key: bool,
    is_main: bool
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowCloseRequestEvent {
    window_id: WindowId
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFullScreenToggleEvent {
    window_id: WindowId,
    is_full_screen: bool
}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    MouseMoved(MouseMovedEvent),
    MouseDown(MouseDownEvent),
    MouseUp(MouseUpEvent),
    // todo mouse enter and exit
    ScrollWheel(ScrollWheelEvent),
    WindowScreenChange(WindowScreenChangeEvent),
    WindowResize(WindowResizeEvent),
    WindowMove(WindowMoveEvent),
    WindowFocusChange(WindowFocusChangeEvent),
    WindowCloseRequest(WindowCloseRequestEvent),
    WindowFullScreenToggle(WindowFullScreenToggleEvent),
    DisplayConfigurationChange,
    ApplicationDidFinishLaunching
}

pub(crate) fn handle_mouse_move(event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let point = unsafe {
            event.locationInWindow()
        };
        let window_id = unsafe {
            event.windowNumber() as WindowId
        };
        let window = unsafe {
            event.window(state.mtm).expect("No window for event")
        };
        // position relative to top left corner of the root view
        let frame = window.contentView().unwrap().frame();

        let event = Event::MouseMoved(MouseMovedEvent {
            window_id,
            point: LogicalPoint {
                x: point.x,
                y: frame.size.height - point.y,
            },
        });
        (state.event_handler)(&event)
    });
    handled
}

trait NSEventExt {
    fn logical_point(&self, mtm: MainThreadMarker) -> LogicalPoint;
    fn window_id(&self) -> WindowId;
}

impl NSEventExt for NSEvent {
    fn logical_point(&self, mtm: MainThreadMarker) -> LogicalPoint {
        let point = unsafe {
            self.locationInWindow()
        };
        let window = unsafe {
            self.window(mtm).expect("No window for event")
        };
        // position relative to top left corner of the root view
        let frame = window.contentView().unwrap().frame();

        LogicalPoint {
            x: point.x,
            y: frame.size.height - point.y,
        }
    }

    fn window_id(&self) -> WindowId {
        unsafe {
            self.windowNumber() as WindowId
        }
    }
}

pub(crate) fn handle_mouse_down(event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseDown(MouseDownEvent {
            window_id: event.window_id(),
            point: event.logical_point(state.mtm),
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_up(event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseUp(MouseUpEvent {
            window_id: event.window_id(),
            point: event.logical_point(state.mtm),
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_window_screen_change(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowScreenChange(WindowScreenChangeEvent {
            window_id: window.window_id(),
            new_screen_id: window.screen().unwrap().screen_id()
        });
        (state.event_handler)(&event)
    });
}

pub (crate) fn handle_window_resize(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowResize(WindowResizeEvent {
            window_id: window.window_id(),
            size: window.get_size()
        });
        (state.event_handler)(&event)
    });
}

pub (crate) fn handle_window_move(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowMove(WindowMoveEvent {
            window_id: window.window_id(),
            origin: window.get_origin()
        });
        (state.event_handler)(&event)
    });
}

pub (crate) fn handle_window_close_request(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowCloseRequest(WindowCloseRequestEvent {
            window_id: window.window_id()
        });
        (state.event_handler)(&event)
    });
}

pub (crate) fn handle_window_focus_change(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowFocusChange(WindowFocusChangeEvent {
            window_id: window.window_id(),
            is_key: window.isKeyWindow(),
            is_main: unsafe { window.isMainWindow() }
        });
        (state.event_handler)(&event)
    });
}

pub (crate) fn handle_window_full_screen_toggle(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowFullScreenToggle(WindowFullScreenToggleEvent {
            window_id: window.window_id(),
            is_full_screen: window.is_full_screen()
        });
        (state.event_handler)(&event)
    });
}

pub (crate) fn handle_display_configuration_change() {
    let _handled = AppState::with(|state| {
        let event = Event::DisplayConfigurationChange;
        (state.event_handler)(&event)
    });
}

pub (crate) fn handle_application_did_finish_launching() {
    let _handled = AppState::with(|state| {
        let event = Event::ApplicationDidFinishLaunching;
        (state.event_handler)(&event)
    });
}