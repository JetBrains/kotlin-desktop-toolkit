use core::f64;

use desktop_common::ffi_utils::BorrowedStrPtr;

use super::{
    keyboard::{KeyCode, KeyModifiersSet},
    mouse::MouseButton,
};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;
pub type Timestamp = f64;

pub type LogicalPixels = f64;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalPoint {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalSize {
    pub width: LogicalPixels,
    pub height: LogicalPixels,
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyDownEvent<'a> {
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: BorrowedStrPtr<'a>,
    pub is_repeat: bool,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent<'a> {
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: BorrowedStrPtr<'a>,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ModifiersChangedEvent {
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDraggedEvent {
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseEnteredEvent {
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseExitedEvent {
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDownEvent {
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseUpEvent {
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    pub scrolling_delta_x: LogicalPixels,
    pub scrolling_delta_y: LogicalPixels,
    pub has_precise_scrolling_deltas: bool,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScreenChangeEvent {
    //    pub new_screen_id: ScreenId,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowResizeEvent {
    pub size: LogicalSize,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowMoveEvent {
    pub origin: LogicalPoint,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFocusChangeEvent {
    pub is_key: bool,
    pub is_main: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowCloseRequestEvent {}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFullScreenToggleEvent {
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

impl Event<'_> {
    //    pub(crate) fn new_window_screen_change_event(window: &NSWindow) -> Self {
    //        Self::WindowScreenChange(WindowScreenChangeEvent {
    //            // todo sometimes it panics when you close the lid
    //            new_screen_id: window.screen().unwrap().screen_id(),
    //        })
    //    }
    //
    //    pub(crate) fn new_window_resize_event(window: &'a NSWindow) -> Self {
    //        Self::WindowResize(WindowResizeEvent {
    //            size: window.get_size(),
    //        })
    //    }
    //
    //    pub(crate) fn new_window_move_event(window: &NSWindow, mtm: MainThreadMarker) -> Self {
    //        Self::WindowMove(WindowMoveEvent {
    //            origin: window.get_origin(mtm).unwrap(), // todo
    //        })
    //    }
    //
    //    pub(crate) fn new_window_close_request_event(window: &NSWindow) -> Self {
    //        Self::WindowCloseRequest(WindowCloseRequestEvent {
    //        })
    //    }
    //
    //    pub(crate) fn new_window_focus_change_event(window: &NSWindow) -> Self {
    //        Self::WindowFocusChange(WindowFocusChangeEvent {
    //
    //            is_key: window.isKeyWindow(),
    //            is_main: unsafe { window.isMainWindow() },
    //        })
    //    }
    //
    //    pub(crate) fn new_window_full_screen_toggle_event(window: &NSWindow) -> Self {
    //        Self::WindowFullScreenToggle(WindowFullScreenToggleEvent {
    //            is_full_screen: window.is_full_screen(),
    //        })
    //    }
    //
    //    pub(crate) fn new_key_down_event(key_info: &'a KeyEventInfo) -> Self {
    //        Self::KeyDown(KeyDownEvent::from_key_event_info(key_info))
    //    }
    //
    //    pub(crate) fn new_key_up_event(key_info: &'a KeyEventInfo) -> Self {
    //        Self::KeyUp(KeyUpEvent {
    //            code: key_info.code,
    //            characters: borrow_ns_string(&key_info.chars),
    //            key: borrow_ns_string(&key_info.chars),
    //            modifiers: key_info.modifiers,
    //            timestamp: key_info.timestamp,
    //        })
    //    }
    //
    //    pub(crate) fn new_modifiers_changed_event(ns_event: &NSEvent) -> Self {
    //        let flags_changed_info = unpack_flags_changed_event(ns_event);
    //        Self::ModifiersChanged(ModifiersChangedEvent {
    //            modifiers: flags_changed_info.modifiers,
    //            code: flags_changed_info.code,
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
    //
    //    pub(crate) fn new_mouse_move_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
    //        Event::MouseMoved(MouseMovedEvent {
    //            location_in_window: ns_event.cursor_location_in_window(mtm),
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
    //
    //    pub(crate) fn new_mouse_drag_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
    //        Event::MouseDragged(MouseDraggedEvent {
    //            button: ns_event.mouse_button().unwrap(),
    //            location_in_window: ns_event.cursor_location_in_window(mtm),
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
    //
    //    pub(crate) fn new_mouse_enter_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
    //        Event::MouseEntered(MouseEnteredEvent {
    //            location_in_window: ns_event.cursor_location_in_window(mtm),
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
    //
    //    pub(crate) fn new_mouse_exit_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
    //        Event::MouseExited(MouseExitedEvent {
    //            location_in_window: ns_event.cursor_location_in_window(mtm),
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
    //
    //    pub(crate) fn new_mouse_down_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
    //        Event::MouseDown(MouseDownEvent {
    //            button: ns_event.mouse_button().unwrap(),
    //            location_in_window: ns_event.cursor_location_in_window(mtm),
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
    //
    //    pub(crate) fn new_mouse_up_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
    //        Event::MouseUp(MouseUpEvent {
    //            button: ns_event.mouse_button().unwrap(),
    //            location_in_window: ns_event.cursor_location_in_window(mtm),
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
    //
    //    pub(crate) fn new_scroll_wheel_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
    //        Event::ScrollWheel(ScrollWheelEvent {
    //            scrolling_delta_x: unsafe { ns_event.scrollingDeltaX() },
    //            scrolling_delta_y: unsafe { ns_event.scrollingDeltaY() },
    //            has_precise_scrolling_deltas: unsafe { ns_event.hasPreciseScrollingDeltas() },
    //            location_in_window: ns_event.cursor_location_in_window(mtm),
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
}
