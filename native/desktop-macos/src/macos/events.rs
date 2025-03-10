#![allow(clippy::let_and_return)]

use core::f64;

use objc2_app_kit::{NSEvent, NSScreen, NSWindow};
use objc2_foundation::MainThreadMarker;

use crate::{
    common::{BorrowedStrPtr, LogicalPixels, LogicalPoint, LogicalSize},
    logger::{PanicDefault, ffi_boundary},
};

use super::{
    keyboard::{EMPTY_KEY_MODIFIERS, KeyCode, KeyEventInfo, KeyModifiersSet, unpack_flags_changed_event},
    mouse::{EmptyMouseButtonsSet, MouseButton, MouseButtonsSet, NSMouseEventExt},
    screen::{NSScreenExts, ScreenId},
    string::borrow_ns_string,
    window::NSWindowExts,
    window_api::WindowId,
};

pub type CallbackUserData = *mut std::ffi::c_void;
// return true if event was handled
pub type EventHandler = extern "C" fn(&Event, CallbackUserData) -> bool;
pub type Timestamp = f64;

#[repr(C)]
#[derive(Debug)]
pub struct KeyDownEvent<'a> {
    pub window_id: WindowId,
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: BorrowedStrPtr<'a>,
    pub is_repeat: bool,
    pub timestamp: Timestamp,
}

impl<'a> KeyDownEvent<'a> {
    pub(crate) fn from_key_event_info(key_info: &'a KeyEventInfo) -> Self {
        Self {
            window_id: key_info.window_id,
            code: key_info.code,
            is_repeat: key_info.is_repeat,
            characters: borrow_ns_string(&key_info.chars),
            key: borrow_ns_string(&key_info.chars),
            modifiers: key_info.modifiers,
            timestamp: key_info.timestamp,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent<'a> {
    pub window_id: WindowId,
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: BorrowedStrPtr<'a>,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ModifiersChangedEvent {
    pub window_id: WindowId,
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDraggedEvent {
    pub window_id: WindowId,
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseEnteredEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseExitedEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDownEvent {
    pub window_id: WindowId,
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseUpEvent {
    pub window_id: WindowId,
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    pub window_id: WindowId,
    pub scrolling_delta_x: LogicalPixels,
    pub scrolling_delta_y: LogicalPixels,
    pub has_precise_scrolling_deltas: bool,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScreenChangeEvent {
    pub window_id: WindowId,
    pub new_screen_id: ScreenId,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowResizeEvent {
    pub window_id: WindowId,
    pub size: LogicalSize,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowMoveEvent {
    pub window_id: WindowId,
    pub origin: LogicalPoint,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFocusChangeEvent {
    pub window_id: WindowId,
    pub is_key: bool,
    pub is_main: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowCloseRequestEvent {
    pub window_id: WindowId,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFullScreenToggleEvent {
    pub window_id: WindowId,
    pub is_full_screen: bool,
}

#[repr(C)]
#[derive(Debug)]
pub enum Event<'a> {
    KeyDown(KeyDownEvent<'a>),
    KeyUp(KeyUpEvent<'a>),
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
}

impl<'a> Event<'a> {
    pub(crate) fn new_window_screen_change_event(window: &NSWindow) -> Self {
        Self::WindowScreenChange(WindowScreenChangeEvent {
            window_id: window.window_id(),
            // todo sometimes it panics when you close the lid
            new_screen_id: window.screen().unwrap().screen_id(),
        })
    }

    pub(crate) fn new_window_resize_event(window: &'a NSWindow) -> Self {
        Self::WindowResize(WindowResizeEvent {
            window_id: window.window_id(),
            size: window.get_size(),
        })
    }

    pub(crate) fn new_window_move_event(window: &NSWindow, mtm: MainThreadMarker) -> Self {
        Self::WindowMove(WindowMoveEvent {
            window_id: window.window_id(),
            origin: window.get_origin(mtm).unwrap(), // todo
        })
    }

    pub(crate) fn new_window_close_request_event(window: &NSWindow) -> Self {
        Self::WindowCloseRequest(WindowCloseRequestEvent {
            window_id: window.window_id(),
        })
    }

    pub(crate) fn new_window_focus_change_event(window: &NSWindow) -> Self {
        Self::WindowFocusChange(WindowFocusChangeEvent {
            window_id: window.window_id(),
            is_key: window.isKeyWindow(),
            is_main: unsafe { window.isMainWindow() },
        })
    }

    pub(crate) fn new_window_full_screen_toggle_event(window: &NSWindow) -> Self {
        Self::WindowFullScreenToggle(WindowFullScreenToggleEvent {
            window_id: window.window_id(),
            is_full_screen: window.is_full_screen(),
        })
    }

    pub(crate) fn new_key_down_event(key_info: &'a KeyEventInfo) -> Self {
        Self::KeyDown(KeyDownEvent::from_key_event_info(key_info))
    }

    pub(crate) fn new_key_up_event(key_info: &'a KeyEventInfo) -> Self {
        Self::KeyUp(KeyUpEvent {
            window_id: key_info.window_id,
            code: key_info.code,
            characters: borrow_ns_string(&key_info.chars),
            key: borrow_ns_string(&key_info.chars),
            modifiers: key_info.modifiers,
            timestamp: key_info.timestamp,
        })
    }

    pub(crate) fn new_modifiers_changed_event(ns_event: &NSEvent) -> Self {
        let flags_changed_info = unpack_flags_changed_event(ns_event);
        Self::ModifiersChanged(ModifiersChangedEvent {
            window_id: ns_event.window_id(),
            modifiers: flags_changed_info.modifiers,
            code: flags_changed_info.code,
            timestamp: unsafe { ns_event.timestamp() },
        })
    }

    pub(crate) fn new_mouse_move_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
        Event::MouseMoved(MouseMovedEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(mtm),
            timestamp: unsafe { ns_event.timestamp() },
        })
    }

    pub(crate) fn new_mouse_drag_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
        Event::MouseDragged(MouseDraggedEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(mtm),
            timestamp: unsafe { ns_event.timestamp() },
        })
    }

    pub(crate) fn new_mouse_enter_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
        Event::MouseEntered(MouseEnteredEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(mtm),
            timestamp: unsafe { ns_event.timestamp() },
        })
    }

    pub(crate) fn new_mouse_exit_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
        Event::MouseExited(MouseExitedEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(mtm),
            timestamp: unsafe { ns_event.timestamp() },
        })
    }

    pub(crate) fn new_mouse_down_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
        Event::MouseDown(MouseDownEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(mtm),
            timestamp: unsafe { ns_event.timestamp() },
        })
    }

    pub(crate) fn new_mouse_up_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
        Event::MouseUp(MouseUpEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(mtm),
            timestamp: unsafe { ns_event.timestamp() },
        })
    }

    pub(crate) fn new_scroll_wheel_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
        Event::ScrollWheel(ScrollWheelEvent {
            window_id: ns_event.window_id(),
            scrolling_delta_x: unsafe { ns_event.scrollingDeltaX() },
            scrolling_delta_y: unsafe { ns_event.scrollingDeltaY() },
            has_precise_scrolling_deltas: unsafe { ns_event.hasPreciseScrollingDeltas() },
            location_in_window: ns_event.cursor_location_in_window(mtm),
            timestamp: unsafe { ns_event.timestamp() },
        })
    }
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
        unsafe { me.windowNumber() }
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
