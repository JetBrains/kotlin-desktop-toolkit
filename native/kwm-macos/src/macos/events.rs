use std::ffi::{CStr, CString};

use log::info;
use objc2_app_kit::{NSEvent, NSEventType, NSWindow};
use objc2_foundation::MainThreadMarker;

use crate::{common::{LogicalPixels, LogicalPoint, LogicalSize, StrPtr}, macos::window};
use anyhow::{anyhow, bail, Result};

use super::{application_api::AppState, keyboard::{unpack_key_event, KeyCode}, screen::{NSScreenExts, ScreenId}, window::NSWindowExts, window_api::WindowId};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;


#[repr(C)]
#[derive(Debug)]
pub struct KeyDownEvent {
    window_id: WindowId,
    code: KeyCode,
    characters: StrPtr,
    key: StrPtr,
    is_repeat: bool,
}

impl Drop for KeyDownEvent {
    fn drop(&mut self) {
        let characters = unsafe {
            CString::from_raw(self.characters)
        };
        std::mem::drop(characters);
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent {
    window_id: WindowId,
    characters: StrPtr,
    key: StrPtr,
    code: KeyCode,
}

impl Drop for KeyUpEvent {
    fn drop(&mut self) {
        let characters = unsafe {
            CString::from_raw(self.characters)
        };
        std::mem::drop(characters);
    }
}

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
    KeyDown(KeyDownEvent),
    KeyUp(KeyUpEvent),
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

pub(crate) fn handle_key_event(ns_event: &NSEvent) -> anyhow::Result<bool> {
    let handled = AppState::with(|state| {
        let window_id = unsafe {
            ns_event.windowNumber() as WindowId
        };

        let event = match unsafe { ns_event.r#type() } {
            NSEventType::KeyDown => {
                let key_info = unpack_key_event(ns_event)?;
                Event::KeyDown(KeyDownEvent {
                    window_id,
                    code: key_info.code,
                    is_repeat: key_info.is_repeat,
                    characters: key_info.chars.into_raw(),
                    key: key_info.key.into_raw(),
                })
            },
            NSEventType::KeyUp => {
                let key_info = unpack_key_event(ns_event)?;
                Event::KeyUp(KeyUpEvent {
                    window_id,
                    code: key_info.code,
                    characters: key_info.chars.into_raw(),
                    key: key_info.key.into_raw()
                })
            },
            _ => bail!("Unexpected type of event {:?}", ns_event)
        };
        Ok((state.event_handler)(&event))
    });
    handled
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
            // todo sometimes it panics when you close the lid
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