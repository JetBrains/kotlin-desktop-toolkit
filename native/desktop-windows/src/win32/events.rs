use super::{
    geometry::{PhysicalPoint, PhysicalSize},
    window_api::WindowId,
};

#[repr(C)]
#[derive(Debug)]
#[allow(dead_code)]
pub enum Event /*<'a>*/ {
    //KeyDown(KeyDownEvent<'a>),
    //KeyUp(KeyUpEvent<'a>),
    //ModifiersChanged(ModifiersChangedEvent),
    //MouseEntered(MouseEnteredEvent),
    //MouseExited(MouseExitedEvent),
    //MouseMoved(MouseMovedEvent),
    //MouseDragged(MouseDraggedEvent),
    //MouseDown(MouseDownEvent),
    //MouseUp(MouseUpEvent),
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
