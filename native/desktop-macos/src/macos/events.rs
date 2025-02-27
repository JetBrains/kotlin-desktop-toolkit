#![allow(clippy::let_and_return)]

use core::f64;

use log::info;
use objc2_app_kit::{NSEvent, NSEventType, NSScreen, NSWindow};
use objc2_foundation::MainThreadMarker;

use crate::{
    common::{ConstStrPtr, LogicalPixels, LogicalPoint, LogicalSize},
    logger::{PanicDefault, ffi_boundary},
};
use anyhow::bail;

use super::{
    application_api::AppState,
    keyboard::{EMPTY_KEY_MODIFIERS, KeyCode, KeyModifiersSet, unpack_flags_changed_event, unpack_key_event},
    mouse::{EmptyMouseButtonsSet, MouseButton, MouseButtonsSet, NSMouseEventExt},
    screen::{NSScreenExts, ScreenId},
    window::NSWindowExts,
    window_api::WindowId,
};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;
pub type Timestamp = f64;

#[repr(C)]
#[derive(Debug)]
pub struct KeyDownEvent {
    window_id: WindowId,
    modifiers: KeyModifiersSet,
    code: KeyCode,
    characters: ConstStrPtr,
    key: ConstStrPtr,
    is_repeat: bool,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent {
    window_id: WindowId,
    modifiers: KeyModifiersSet,
    code: KeyCode,
    characters: ConstStrPtr,
    key: ConstStrPtr,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ModifiersChangedEvent {
    window_id: WindowId,
    modifiers: KeyModifiersSet,
    code: KeyCode,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    window_id: WindowId,
    location_in_window: LogicalPoint,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDraggedEvent {
    window_id: WindowId,
    button: MouseButton,
    location_in_window: LogicalPoint,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseEnteredEvent {
    window_id: WindowId,
    location_in_window: LogicalPoint,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseExitedEvent {
    window_id: WindowId,
    location_in_window: LogicalPoint,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDownEvent {
    window_id: WindowId,
    button: MouseButton,
    location_in_window: LogicalPoint,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseUpEvent {
    window_id: WindowId,
    button: MouseButton,
    location_in_window: LogicalPoint,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    window_id: WindowId,
    scrolling_delta_x: LogicalPixels,
    scrolling_delta_y: LogicalPixels,
    has_precise_scrolling_deltas: bool,
    location_in_window: LogicalPoint,
    timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScreenChangeEvent {
    window_id: WindowId,
    new_screen_id: ScreenId,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowResizeEvent {
    window_id: WindowId,
    size: LogicalSize,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowMoveEvent {
    window_id: WindowId,
    origin: LogicalPoint,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFocusChangeEvent {
    window_id: WindowId,
    is_key: bool,
    is_main: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowCloseRequestEvent {
    window_id: WindowId,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFullScreenToggleEvent {
    window_id: WindowId,
    is_full_screen: bool,
}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    KeyDown(KeyDownEvent),
    KeyUp(KeyUpEvent),
    ModifiersChanged(ModifiersChangedEvent),
    MouseMoved(MouseMovedEvent),
    MouseDragged(MouseDraggedEvent),
    MouseEntered(MouseEnteredEvent),
    MouseExited(MouseExitedEvent),
    MouseDown(MouseDownEvent),
    MouseUp(MouseUpEvent),
    ScrollWheel(ScrollWheelEvent),
    WindowScreenChange(WindowScreenChangeEvent),
    WindowResize(WindowResizeEvent),
    WindowMove(WindowMoveEvent),
    WindowFocusChange(WindowFocusChangeEvent),
    WindowCloseRequest(WindowCloseRequestEvent),
    WindowFullScreenToggle(WindowFullScreenToggleEvent),
    DisplayConfigurationChange,
    ApplicationDidFinishLaunching,
}

impl PanicDefault for MouseButtonsSet {
    fn default() -> Self {
        EmptyMouseButtonsSet
    }
}

#[unsafe(no_mangle)]
extern "C" fn events_pressed_mouse_buttons() -> MouseButtonsSet {
    ffi_boundary("events_pressed_mouse_buttons", || Ok(NSEvent::pressed_mouse_buttons()))
}

impl PanicDefault for KeyModifiersSet {
    fn default() -> Self {
        EMPTY_KEY_MODIFIERS
    }
}

#[unsafe(no_mangle)]
extern "C" fn events_pressed_modifiers() -> KeyModifiersSet {
    ffi_boundary("events_pressed_modifiers", || Ok(NSEvent::pressed_modifiers()))
}

#[unsafe(no_mangle)]
extern "C" fn events_cursor_location_in_screen() -> LogicalPoint {
    ffi_boundary("events_cursor_location_in_screen", || {
        let mtm = MainThreadMarker::new().unwrap();
        Ok(NSEvent::cursor_location_in_screen(mtm))
    })
}

trait NSEventExt {
    fn me(&self) -> &NSEvent;

    fn window_id(&self) -> WindowId {
        let me = self.me();
        unsafe { me.windowNumber() as WindowId }
    }

    fn cursor_location_in_window(&self, mtm: MainThreadMarker) -> LogicalPoint {
        let me = self.me();
        let point = unsafe {
            // position is relative to bottom left corner of the root view
            me.locationInWindow()
        };
        let window = unsafe { me.window(mtm).expect("No window for event") };
        let frame = window.contentView().unwrap().frame();
        LogicalPoint::from_macos_coords(point, frame.size.height)
    }

    fn cursor_location_in_screen(mtm: MainThreadMarker) -> LogicalPoint {
        let point = unsafe { NSEvent::mouseLocation() };
        let screen = NSScreen::primary(mtm).unwrap();
        LogicalPoint::from_macos_coords(point, screen.height())
    }

    fn pressed_modifiers() -> KeyModifiersSet {
        unsafe { NSEvent::modifierFlags_class() }.into()
    }
}

impl NSEventExt for NSEvent {
    fn me(&self) -> &NSEvent {
        self
    }
}

pub(crate) fn handle_key_event(ns_event: &NSEvent) -> anyhow::Result<bool> {
    let handled = AppState::with(|state| {
        let key_info = unpack_key_event(ns_event)?;
        let event = match unsafe { ns_event.r#type() } {
            NSEventType::KeyDown => Event::KeyDown(KeyDownEvent {
                window_id: ns_event.window_id(),
                code: key_info.code,
                is_repeat: key_info.is_repeat,
                characters: key_info.chars.UTF8String(),
                key: key_info.chars.UTF8String(),
                modifiers: key_info.modifiers,
                timestamp: unsafe { ns_event.timestamp() },
            }),
            NSEventType::KeyUp => Event::KeyUp(KeyUpEvent {
                window_id: ns_event.window_id(),
                code: key_info.code,
                characters: key_info.chars.UTF8String(),
                key: key_info.chars.UTF8String(),
                modifiers: key_info.modifiers,
                timestamp: unsafe { ns_event.timestamp() },
            }),
            _ => bail!("Unexpected type of event {:?}", ns_event),
        };
        Ok((state.event_handler)(&event))
    });
    handled
}

pub(crate) fn handle_flags_changed_event(ns_event: &NSEvent) -> anyhow::Result<bool> {
    let handled = AppState::with(|state| {
        let flags_changed_info = unpack_flags_changed_event(ns_event)?;
        let event = Event::ModifiersChanged(ModifiersChangedEvent {
            window_id: ns_event.window_id(),
            modifiers: flags_changed_info.modifiers,
            code: flags_changed_info.code,
            timestamp: unsafe { ns_event.timestamp() },
        });

        Ok((state.event_handler)(&event))
    });
    handled
}

pub(crate) fn handle_mouse_move(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseMoved(MouseMovedEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_drag(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseDragged(MouseDraggedEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_enter(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseEntered(MouseEnteredEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_exit(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseExited(MouseExitedEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_down(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseDown(MouseDownEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_up(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseUp(MouseUpEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_scroll_wheel(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::ScrollWheel(ScrollWheelEvent {
            window_id: ns_event.window_id(),
            scrolling_delta_x: unsafe { ns_event.scrollingDeltaX() },
            scrolling_delta_y: unsafe { ns_event.scrollingDeltaY() },
            has_precise_scrolling_deltas: unsafe { ns_event.hasPreciseScrollingDeltas() },
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
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
            new_screen_id: window.screen().unwrap().screen_id(),
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_resize(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowResize(WindowResizeEvent {
            window_id: window.window_id(),
            size: window.get_size(),
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_move(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowMove(WindowMoveEvent {
            window_id: window.window_id(),
            origin: window.get_origin(state.mtm).unwrap(), // todo
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_close_request(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowCloseRequest(WindowCloseRequestEvent {
            window_id: window.window_id(),
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_focus_change(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowFocusChange(WindowFocusChangeEvent {
            window_id: window.window_id(),
            is_key: window.isKeyWindow(),
            is_main: unsafe { window.isMainWindow() },
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_full_screen_toggle(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowFullScreenToggle(WindowFullScreenToggleEvent {
            window_id: window.window_id(),
            is_full_screen: window.is_full_screen(),
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_display_configuration_change() {
    let _handled = AppState::with(|state| {
        let event = Event::DisplayConfigurationChange;
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_application_did_finish_launching() {
    let _handled = AppState::with(|state| {
        let event = Event::ApplicationDidFinishLaunching;
        (state.event_handler)(&event)
    });
}
