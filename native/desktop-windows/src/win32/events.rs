use super::{geometry::PhysicalSize, window_api::WindowId};

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
    //WindowScaleChanged(WindowScaleChangedEvent),
    //WindowScreenChange(WindowScreenChangeEvent),
    //WindowResize(WindowResizeEvent),
    //WindowMove(WindowMoveEvent),
}

// return true if event was handled
pub type EventHandler = extern "C" fn(WindowId, &Event) -> bool;

#[repr(C)]
#[derive(Debug)]
pub struct WindowDrawEvent {
    pub physical_size: PhysicalSize,
    pub scale: f32,
}

impl From<WindowDrawEvent> for Event /*<'_>*/ {
    fn from(value: WindowDrawEvent) -> Self {
        Self::WindowDraw(value)
    }
}
