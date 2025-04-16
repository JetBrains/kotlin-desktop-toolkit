use core::f64;
use std::ffi::CString;

use desktop_common::ffi_utils::BorrowedStrPtr;
use smithay_client_toolkit::{
    reexports::client::{Proxy, protocol::wl_output::WlOutput},
    seat::{
        keyboard::{KeyEvent, Modifiers},
        pointer::{AxisScroll, PointerEvent},
    },
};

use super::geometry::{LogicalPixels, LogicalPoint, LogicalSize, PhysicalSize};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event, WindowId) -> bool;

pub(crate) type InternalEventHandler = dyn Fn(&Event) -> bool;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Timestamp(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct ScreenId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WindowId(pub u32);

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct MouseButton(pub u32);

#[derive(Debug)]
#[repr(transparent)]
pub struct MouseButtonsSet(pub u32);

#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct KeyModifiers {
    /// The "control" key
    pub ctrl: bool,

    /// The "alt" key
    pub alt: bool,

    /// The "shift" key
    pub shift: bool,

    /// The "Caps lock" key
    pub caps_lock: bool,

    /// The "logo" key
    ///
    /// Also known as the "windows" or "super" key on a keyboard.
    pub logo: bool,

    /// The "Num lock" key
    pub num_lock: bool,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct KeyCode(pub u32);

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum WindowResizeEdge {
    /// Nothing is being dragged.
    None,
    /// The top edge is being dragged.
    Top,
    /// The bottom edge is being dragged.
    Bottom,
    /// The left edge is being dragged.
    Left,
    /// The top left corner is being dragged.
    TopLeft,
    /// The bottom left corner is being dragged.
    BottomLeft,
    /// The right edge is being dragged.
    Right,
    /// The top right corner is being dragged.
    TopRight,
    /// The bottom right corner is being dragged.
    BottomRight,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum WindowFrameAction {
    None,
    /// The window should be minimized.
    Minimize,
    /// The window should be maximized.
    Maximize,
    /// The window should be unmaximized.
    UnMaximize,
    /// The window should be closed.
    Close,
    /// An interactive move should be started.
    Move,
    /// An interactive resize should be started with the provided edge.
    Resize(WindowResizeEdge),
    /// Show window menu.
    ///
    /// The coordinates are relative to the base surface, as in should be
    /// directly passed to the `xdg_toplevel::show_window_menu`.
    ShowMenu(i32, i32),
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyDownEvent<'a> {
    pub modifiers: KeyModifiers,
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: BorrowedStrPtr<'a>,
    pub is_repeat: bool,
    pub timestamp: Timestamp,
    pub frame_action_out: WindowFrameAction,
}

impl<'a> From<&'a KeyDownEvent<'a>> for Event<'a> {
    fn from(value: &'a KeyDownEvent<'a>) -> Self {
        Self::KeyDown(value)
    }
}

impl<'a> KeyDownEvent<'a> {
    pub(crate) fn new(event: &KeyEvent, characters: Option<&'a CString>, key: Option<&'a CString>) -> Self {
        Self {
            modifiers: KeyModifiers::default(), // TODO
            code: KeyCode(event.raw_code),
            characters: BorrowedStrPtr::new_optional(characters),
            key: BorrowedStrPtr::new_optional(key),
            is_repeat: false,        // TODO
            timestamp: Timestamp(0), // TODO
            frame_action_out: WindowFrameAction::None,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent<'a> {
    pub modifiers: KeyModifiers,
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: BorrowedStrPtr<'a>,
    pub timestamp: Timestamp,
}

impl<'a> From<KeyUpEvent<'a>> for Event<'a> {
    fn from(value: KeyUpEvent<'a>) -> Self {
        Self::KeyUp(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ModifiersChangedEvent {
    pub modifiers: KeyModifiers,
    pub timestamp: Timestamp,
}

impl From<ModifiersChangedEvent> for Event<'_> {
    fn from(value: ModifiersChangedEvent) -> Self {
        Self::ModifiersChanged(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl From<MouseMovedEvent> for Event<'_> {
    fn from(value: MouseMovedEvent) -> Self {
        Self::MouseMoved(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDraggedEvent {
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl From<MouseDraggedEvent> for Event<'_> {
    fn from(value: MouseDraggedEvent) -> Self {
        Self::MouseDragged(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseEnteredEvent {
    pub location_in_window: LogicalPoint,
    //    pub timestamp: Timestamp,
}

impl From<MouseEnteredEvent> for Event<'_> {
    fn from(value: MouseEnteredEvent) -> Self {
        Self::MouseEntered(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseExitedEvent {
    pub location_in_window: LogicalPoint,
    //    pub timestamp: Timestamp,
}

impl From<MouseExitedEvent> for Event<'_> {
    fn from(value: MouseExitedEvent) -> Self {
        Self::MouseExited(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDownEvent {
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
    pub frame_action_out: WindowFrameAction,
}

impl<'a> From<&'a MouseDownEvent> for Event<'a> {
    fn from(value: &'a MouseDownEvent) -> Self {
        Self::MouseDown(value)
    }
}

impl MouseDownEvent {
    pub(crate) const fn new(event: &PointerEvent, button: u32, time: u32) -> Self {
        Self {
            button: MouseButton(button),
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
            timestamp: Timestamp(time),
            frame_action_out: WindowFrameAction::None,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseUpEvent {
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl From<MouseUpEvent> for Event<'_> {
    fn from(value: MouseUpEvent) -> Self {
        Self::MouseUp(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    pub scrolling_delta_x: LogicalPixels,
    pub scrolling_delta_y: LogicalPixels,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl From<ScrollWheelEvent> for Event<'_> {
    fn from(value: ScrollWheelEvent) -> Self {
        Self::ScrollWheel(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScreenChangeEvent {
    pub new_screen_id: ScreenId,
}

impl From<WindowScreenChangeEvent> for Event<'_> {
    fn from(value: WindowScreenChangeEvent) -> Self {
        Self::WindowScreenChange(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowCapabilities {
    /// `show_window_menu` is available.
    pub window_menu: bool,

    /// Window can be maximized and unmaximized.
    pub maximixe: bool,

    /// Window can be fullscreened and unfullscreened.
    pub fullscreen: bool,

    /// Window can be minimized.
    pub minimize: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowResizeEvent {
    pub size: LogicalSize,
    pub maximized: bool,
    pub fullscreen: bool,
    pub client_side_decorations: bool,
    pub capabilities: WindowCapabilities,
}

impl From<WindowResizeEvent> for Event<'_> {
    fn from(value: WindowResizeEvent) -> Self {
        Self::WindowResize(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFocusChangeEvent {
    pub is_key: bool,
    pub is_main: bool,
}

impl From<WindowFocusChangeEvent> for Event<'_> {
    fn from(value: WindowFocusChangeEvent) -> Self {
        Self::WindowFocusChange(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFullScreenToggleEvent {
    pub is_full_screen: bool,
}

impl From<WindowFullScreenToggleEvent> for Event<'_> {
    fn from(value: WindowFullScreenToggleEvent) -> Self {
        Self::WindowFullScreenToggle(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct SoftwareDrawData {
    pub canvas: *mut u8,
    pub stride: i32,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowDrawEvent {
    pub software_draw_data: SoftwareDrawData,
    pub physical_size: PhysicalSize,
    pub scale: f64,
}

impl From<WindowDrawEvent> for Event<'_> {
    fn from(value: WindowDrawEvent) -> Self {
        Self::WindowDraw(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScaleChangedEvent {
    pub new_scale: f64,
}

impl From<WindowScaleChangedEvent> for Event<'_> {
    fn from(value: WindowScaleChangedEvent) -> Self {
        Self::WindowScaleChanged(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum Event<'a> {
    KeyDown(&'a KeyDownEvent<'a>),
    KeyUp(KeyUpEvent<'a>),
    ModifiersChanged(ModifiersChangedEvent),
    MouseMoved(MouseMovedEvent),
    MouseDragged(MouseDraggedEvent),
    MouseEntered(MouseEnteredEvent),
    MouseExited(MouseExitedEvent),
    MouseDown(&'a MouseDownEvent),
    MouseUp(MouseUpEvent),
    ScrollWheel(ScrollWheelEvent),
    WindowScreenChange(WindowScreenChangeEvent),
    WindowResize(WindowResizeEvent),
    WindowFocusChange(WindowFocusChangeEvent),
    WindowCloseRequest,
    WindowFullScreenToggle(WindowFullScreenToggleEvent),
    WindowDraw(WindowDrawEvent),
    WindowScaleChanged(WindowScaleChangedEvent),
}

impl Event<'_> {
    pub(crate) fn new_window_screen_change_event(output: &WlOutput) -> Self {
        WindowScreenChangeEvent {
            new_screen_id: ScreenId(output.id().protocol_id()),
        }
        .into()
    }

    pub(crate) fn new_window_focus_change_event(is_key: bool) -> Self {
        WindowFocusChangeEvent { is_key, is_main: is_key }.into()
    }

    //    pub(crate) fn new_window_full_screen_toggle_event(window: &NSWindow) -> Self {
    //        Self::WindowFullScreenToggle(WindowFullScreenToggleEvent {
    //            is_full_screen: window.is_full_screen(),
    //        })
    //    }

    pub(crate) fn new_key_up_event<'a>(event: &KeyEvent, characters: Option<&'a CString>, key: Option<&'a CString>) -> Event<'a> {
        Event::KeyUp(KeyUpEvent {
            modifiers: KeyModifiers::default(), // TODO
            code: KeyCode(event.raw_code),
            characters: BorrowedStrPtr::new_optional(characters),
            key: BorrowedStrPtr::new_optional(key),
            timestamp: Timestamp(0), // TODO
        })
    }

    pub(crate) const fn new_modifiers_changed_event(modifiers: Modifiers) -> Self {
        let key_modifiers = KeyModifiers {
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            caps_lock: modifiers.caps_lock,
            logo: modifiers.logo,
            num_lock: modifiers.num_lock,
        };
        Self::ModifiersChanged(ModifiersChangedEvent {
            modifiers: key_modifiers,
            timestamp: Timestamp(0), // TODO
        })
    }

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

    pub(crate) const fn new_window_scale_changed_event(new_scale: f64) -> Self {
        Event::WindowScaleChanged(WindowScaleChangedEvent { new_scale })
    }
}
