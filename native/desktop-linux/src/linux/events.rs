use core::f64;

use desktop_common::ffi_utils::BorrowedStrPtr;
use smithay_client_toolkit::{
    reexports::client::{Proxy, protocol::wl_output::WlOutput},
    seat::pointer::{AxisScroll, PointerEvent},
};

use super::{
    keyboard::{KeyCode, KeyModifiersSet},
    mouse::MouseButton,
};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;

pub type InternalEventHandler = dyn FnMut(&Event) -> bool;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Timestamp(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalPixels(pub f64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct ScreenId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WindowId(pub u32);

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
    //    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseExitedEvent {
    pub location_in_window: LogicalPoint,
    //    pub timestamp: Timestamp,
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
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScreenChangeEvent {
    pub new_screen_id: ScreenId,
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
pub struct WindowFullScreenToggleEvent {
    pub is_full_screen: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowDrawEvent {
    pub buffer: *mut u8,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
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
    WindowCloseRequest,
    WindowFullScreenToggle(WindowFullScreenToggleEvent),
    WindowDraw(WindowDrawEvent),
}

impl Event<'_> {
    pub(crate) fn new_window_screen_change_event(output: &WlOutput) -> Self {
        Self::WindowScreenChange(WindowScreenChangeEvent {
            new_screen_id: ScreenId(output.id().protocol_id()),
        })
    }

    pub(crate) const fn new_window_resize_event(size: LogicalSize) -> Self {
        Self::WindowResize(WindowResizeEvent { size })
    }

    //        pub(crate) fn new_window_move_event(window: &NSWindow) -> Self {
    //            Self::WindowMove(WindowMoveEvent {
    //                origin: window.get_origin(mtm).unwrap(), // todo
    //            })
    //        }

    pub(crate) const fn new_window_focus_change_event(is_key: bool) -> Self {
        Self::WindowFocusChange(WindowFocusChangeEvent { is_key, is_main: is_key })
    }

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
    pub(crate) const fn new_mouse_move_event(event: &PointerEvent, time: u32) -> Self {
        Event::MouseMoved(MouseMovedEvent {
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
            timestamp: Timestamp(time),
        })
    }
    //
    //    pub(crate) fn new_mouse_drag_event(ns_event: &NSEvent, mtm: MainThreadMarker) -> Self {
    //        Event::MouseDragged(MouseDraggedEvent {
    //            button: ns_event.mouse_button().unwrap(),
    //            location_in_window: ns_event.cursor_location_in_window(mtm),
    //            timestamp: unsafe { ns_event.timestamp() },
    //        })
    //    }
    //
    pub(crate) const fn new_mouse_enter_event(event: &PointerEvent) -> Self {
        Event::MouseEntered(MouseEnteredEvent {
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
        })
    }

    pub(crate) const fn new_mouse_exit_event(event: &PointerEvent) -> Self {
        Event::MouseExited(MouseExitedEvent {
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
        })
    }

    pub(crate) const fn new_mouse_down_event(event: &PointerEvent, button: u32, time: u32) -> Self {
        Event::MouseDown(MouseDownEvent {
            button: MouseButton(button),
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
            timestamp: Timestamp(time),
        })
    }

    pub(crate) const fn new_mouse_up_event(event: &PointerEvent, button: u32, time: u32) -> Self {
        Event::MouseUp(MouseUpEvent {
            button: MouseButton(button),
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
            timestamp: Timestamp(time),
        })
    }

    pub(crate) const fn new_scroll_wheel_event(event: &PointerEvent, time: u32, horizontal: AxisScroll, vertical: AxisScroll) -> Self {
        Event::ScrollWheel(ScrollWheelEvent {
            scrolling_delta_x: LogicalPixels(horizontal.absolute),
            scrolling_delta_y: LogicalPixels(vertical.absolute),
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
            timestamp: Timestamp(time),
        })
    }

    pub(crate) const fn new_window_draw_event(buffer: &mut [u8], width: i32, height: i32, stride: i32) -> Self {
        Event::WindowDraw(WindowDrawEvent {
            buffer: buffer.as_mut_ptr(),
            width,
            height,
            stride,
        })
    }
}
