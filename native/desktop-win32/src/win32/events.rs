use desktop_common::ffi_utils::RustAllocatedStrPtr;

use super::{
    geometry::{LogicalPoint, PhysicalPoint, PhysicalSize},
    keyboard::{PhysicalKeyStatus, VirtualKey},
    mouse::{MouseButton, MouseKeyState},
    window_api::WindowId,
};

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    CharacterReceived(CharacterReceivedEvent),
    MouseEntered(MouseEnteredEvent),
    MouseExited(MouseExitedEvent),
    MouseMoved(MouseMovedEvent),
    MouseDown(MouseButtonEvent),
    MouseUp(MouseButtonEvent),
    NCHitTest(NCHitTestEvent),
    ScrollWheelX(ScrollWheelEvent),
    ScrollWheelY(ScrollWheelEvent),
    WindowCloseRequest,
    WindowDraw(WindowDrawEvent),
    //WindowFocusChange(WindowFocusChangeEvent),
    WindowKeyboardEnter,
    WindowKeyboardLeave,
    //WindowFullScreenToggle(WindowFullScreenToggleEvent),
    WindowScaleChanged(WindowScaleChangedEvent),
    //WindowScreenChange(WindowScreenChangeEvent),
    WindowResize(WindowResizeEvent),
    //WindowMove(WindowMoveEvent),
}

// return true if event was handled
pub type EventHandler = extern "C" fn(WindowId, &Event) -> bool;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Timestamp(pub u64);

#[repr(C)]
#[derive(Debug)]
pub struct WindowDrawEvent {
    pub size: PhysicalSize,
    pub scale: f32,
}

impl From<WindowDrawEvent> for Event {
    fn from(value: WindowDrawEvent) -> Self {
        Self::WindowDraw(value)
    }
}

#[repr(C)]
#[derive(Debug)]
#[allow(clippy::struct_field_names)]
pub struct WindowScaleChangedEvent {
    pub new_origin: PhysicalPoint,
    pub new_size: PhysicalSize,
    pub new_scale: f32,
}

impl From<WindowScaleChangedEvent> for Event {
    fn from(value: WindowScaleChangedEvent) -> Self {
        Self::WindowScaleChanged(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowResizeEvent {
    pub size: PhysicalSize,
    pub scale: f32,
    pub kind: WindowResizeKind,
}

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum WindowResizeKind {
    Restored,
    Maximized,
    Minimized,
    Other(u32),
}

impl From<WindowResizeEvent> for Event {
    fn from(value: WindowResizeEvent) -> Self {
        Self::WindowResize(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct NCHitTestEvent {
    pub mouse_x: i32,
    pub mouse_y: i32,
}

impl From<NCHitTestEvent> for Event {
    fn from(value: NCHitTestEvent) -> Self {
        Self::NCHitTest(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyEvent {
    pub key_code: VirtualKey,
    pub key_status: PhysicalKeyStatus,
    pub is_system_key: bool,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct CharacterReceivedEvent {
    pub key_code: u16,
    pub characters: RustAllocatedStrPtr,
    pub key_status: PhysicalKeyStatus,
    pub is_dead_char: bool,
    pub is_system_key: bool,
}

impl From<CharacterReceivedEvent> for Event {
    fn from(value: CharacterReceivedEvent) -> Self {
        Self::CharacterReceived(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseEnteredEvent {
    pub key_state: MouseKeyState,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl From<MouseEnteredEvent> for Event {
    fn from(value: MouseEnteredEvent) -> Self {
        Self::MouseEntered(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseExitedEvent {
    pub timestamp: Timestamp,
}

impl From<MouseExitedEvent> for Event {
    fn from(value: MouseExitedEvent) -> Self {
        Self::MouseExited(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    pub key_state: MouseKeyState,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl From<MouseMovedEvent> for Event {
    fn from(value: MouseMovedEvent) -> Self {
        Self::MouseMoved(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseButtonEvent {
    pub button: MouseButton,
    pub key_state: MouseKeyState,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    pub scrolling_delta: u16,
    pub key_state: MouseKeyState,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}
