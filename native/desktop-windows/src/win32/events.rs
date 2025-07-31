use desktop_common::ffi_utils::RustAllocatedStrPtr;

use super::{
    geometry::{PhysicalPoint, PhysicalSize},
    keyboard::{PhysicalKeyStatus, VirtualKey},
    window_api::WindowId,
};

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    CharacterReceived(CharacterReceivedEvent),
    //MouseEntered(MouseEnteredEvent),
    //MouseExited(MouseExitedEvent),
    //MouseMoved(MouseMovedEvent),
    //MouseDragged(MouseDraggedEvent),
    //MouseDown(MouseDownEvent),
    //MouseUp(MouseUpEvent),
    NCHitTest(NCHitTestEvent),
    //ScrollWheel(ScrollWheelEvent),
    WindowCloseRequest,
    WindowDraw(WindowDrawEvent),
    //WindowFocusChange(WindowFocusChangeEvent),
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
