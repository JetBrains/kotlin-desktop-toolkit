use desktop_common::ffi_utils::AutoDropStrPtr;

use super::{
    appearance::Appearance,
    geometry::{LogicalPoint, PhysicalPoint, PhysicalSize},
    keyboard::{PhysicalKeyStatus, VirtualKey},
    pointer::{PointerButton, PointerButtonChange, PointerState},
    window::WindowId,
};

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event {
    CharacterReceived(CharacterReceivedEvent),
    KeyDown(KeyEvent),
    KeyUp(KeyEvent),
    NCCalcSize(NCCalcSizeEvent),
    NCHitTest(NCHitTestEvent),
    PointerDown(PointerDownEvent),
    PointerEntered(PointerEnteredEvent),
    PointerExited(PointerExitedEvent),
    PointerUpdated(PointerUpdatedEvent),
    PointerUp(PointerUpEvent),
    ScrollWheelX(ScrollWheelEvent),
    ScrollWheelY(ScrollWheelEvent),
    SystemAppearanceChange(SystemAppearanceChangeEvent),
    WindowCloseRequest,
    WindowDraw(WindowDrawEvent),
    //WindowFocusChange(WindowFocusChangeEvent),
    //WindowFullScreenToggle(WindowFullScreenToggleEvent),
    WindowKeyboardEnter,
    WindowKeyboardLeave,
    WindowMove(WindowMoveEvent),
    WindowResize(WindowResizeEvent),
    WindowScaleChanged(WindowScaleChangedEvent),
    WindowTitleChanged(WindowTitleChangedEvent),
}

// return true if event was handled
pub type EventHandler = extern "C" fn(WindowId, &Event) -> bool;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Timestamp(pub u64);

#[repr(C)]
#[derive(Debug)]
pub struct CharacterReceivedEvent {
    pub key_code: u16,
    pub characters: AutoDropStrPtr,
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
pub struct KeyEvent {
    pub key_code: VirtualKey,
    pub key_status: PhysicalKeyStatus,
    pub is_system_key: bool,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct NCCalcSizeEvent {
    pub origin: PhysicalPoint,
    pub size: PhysicalSize,
    pub scale: f32,
}

impl From<NCCalcSizeEvent> for Event {
    fn from(value: NCCalcSizeEvent) -> Self {
        Self::NCCalcSize(value)
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
pub struct PointerDownEvent {
    pub button: PointerButton,
    pub click_count: u32,
    pub location_in_window: LogicalPoint,
    pub non_client_area: bool,
    pub state: PointerState,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct PointerEnteredEvent {
    pub location_in_window: LogicalPoint,
    pub state: PointerState,
    pub timestamp: Timestamp,
}

impl From<PointerEnteredEvent> for Event {
    fn from(value: PointerEnteredEvent) -> Self {
        Self::PointerEntered(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PointerExitedEvent {
    pub location_in_window: LogicalPoint,
    pub state: PointerState,
    pub timestamp: Timestamp,
}

impl From<PointerExitedEvent> for Event {
    fn from(value: PointerExitedEvent) -> Self {
        Self::PointerExited(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PointerUpdatedEvent {
    pub button_change: PointerButtonChange,
    pub location_in_window: LogicalPoint,
    pub non_client_area: bool,
    pub state: PointerState,
    pub timestamp: Timestamp,
}

impl From<PointerUpdatedEvent> for Event {
    fn from(value: PointerUpdatedEvent) -> Self {
        Self::PointerUpdated(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct PointerUpEvent {
    pub button: PointerButton,
    pub location_in_window: LogicalPoint,
    pub non_client_area: bool,
    pub state: PointerState,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    pub scrolling_delta: i32,
    pub location_in_window: LogicalPoint,
    pub state: PointerState,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct SystemAppearanceChangeEvent {
    pub new_appearance: Appearance,
}

impl From<SystemAppearanceChangeEvent> for Event {
    fn from(value: SystemAppearanceChangeEvent) -> Self {
        Self::SystemAppearanceChange(value)
    }
}

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
pub struct WindowMoveEvent {
    pub origin: PhysicalPoint,
    pub scale: f32,
}

impl From<WindowMoveEvent> for Event {
    fn from(value: WindowMoveEvent) -> Self {
        Self::WindowMove(value)
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
pub struct WindowScaleChangedEvent {
    pub origin: PhysicalPoint,
    pub size: PhysicalSize,
    pub scale: f32,
}

impl From<WindowScaleChangedEvent> for Event {
    fn from(value: WindowScaleChangedEvent) -> Self {
        Self::WindowScaleChanged(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowTitleChangedEvent {
    pub title: AutoDropStrPtr,
}

impl From<WindowTitleChangedEvent> for Event {
    fn from(value: WindowTitleChangedEvent) -> Self {
        Self::WindowTitleChanged(value)
    }
}
