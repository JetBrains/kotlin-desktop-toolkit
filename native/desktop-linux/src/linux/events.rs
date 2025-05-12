use core::f64;
use std::ffi::{CStr, CString};

use desktop_common::ffi_utils::{BorrowedArray, BorrowedStrPtr};
use smithay_client_toolkit::{
    reexports::client::{Proxy, protocol::wl_output::WlOutput},
    seat::{
        keyboard::Modifiers,
        pointer::{AxisScroll, PointerEvent},
    },
};

use crate::linux::{
    data_transfer::DataTransferContentInternal,
    geometry::{LogicalPixels, LogicalPoint, LogicalSize, PhysicalSize},
};

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
pub struct WindowId(pub i64);

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
#[derive(Debug)]
pub struct DataTransferContent<'a> {
    pub serial: i32,
    pub data: BorrowedArray<'a, u8>,
    pub mime_types: BorrowedStrPtr<'a>,
}

impl<'a> From<DataTransferContent<'a>> for Event<'a> {
    fn from(value: DataTransferContent<'a>) -> Self {
        Self::DataTransfer(value)
    }
}

impl<'a> DataTransferContent<'a> {
    #[must_use]
    pub fn new(serial: i32, data: &'a [u8], mime_types: &'a CStr) -> Self {
        Self {
            serial,
            data: BorrowedArray::from_slice(data),
            mime_types: BorrowedStrPtr::new(mime_types),
        }
    }

    pub fn to_internal(&self) -> anyhow::Result<DataTransferContentInternal> {
        Ok(DataTransferContentInternal::new(self.data.as_slice()?, self.mime_types.as_str()?))
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyDownEvent<'a> {
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: u32,
    pub is_repeat: bool,
}

impl<'a> From<KeyDownEvent<'a>> for Event<'a> {
    fn from(value: KeyDownEvent<'a>) -> Self {
        Self::KeyDown(value)
    }
}

impl<'a> KeyDownEvent<'a> {
    pub(crate) fn new(code: KeyCode, key: u32, characters: Option<&'a CString>) -> Self {
        Self {
            code,
            characters: BorrowedStrPtr::new_optional(characters),
            key,
            is_repeat: false, // TODO
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent<'a> {
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: u32,
}

impl<'a> KeyUpEvent<'a> {
    pub(crate) fn new(code: KeyCode, key: u32, characters: Option<&'a CString>) -> Self {
        Self {
            code,
            characters: BorrowedStrPtr::new_optional(characters),
            key,
        }
    }
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
}

impl ModifiersChangedEvent {
    pub(crate) const fn new(modifiers: Modifiers) -> Self {
        let key_modifiers = KeyModifiers {
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            caps_lock: modifiers.caps_lock,
            logo: modifiers.logo,
            num_lock: modifiers.num_lock,
        };
        Self { modifiers: key_modifiers }
    }
}

impl From<ModifiersChangedEvent> for Event<'_> {
    fn from(value: ModifiersChangedEvent) -> Self {
        Self::ModifiersChanged(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseEnteredEvent {
    pub location_in_window: LogicalPoint,
}

impl MouseEnteredEvent {
    pub(crate) const fn new(event: &PointerEvent) -> Self {
        Self {
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
        }
    }
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
}

impl MouseExitedEvent {
    pub(crate) const fn new(event: &PointerEvent) -> Self {
        Self {
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
        }
    }
}

impl From<MouseExitedEvent> for Event<'_> {
    fn from(value: MouseExitedEvent) -> Self {
        Self::MouseExited(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl MouseMovedEvent {
    pub(crate) const fn new(event: &PointerEvent, time: u32) -> Self {
        Self {
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
            timestamp: Timestamp(time),
        }
    }
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
pub struct MouseDownEvent {
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl From<MouseDownEvent> for Event<'_> {
    fn from(value: MouseDownEvent) -> Self {
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

impl MouseUpEvent {
    pub(crate) const fn new(event: &PointerEvent, button: u32, time: u32) -> Self {
        Self {
            button: MouseButton(button),
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
            timestamp: Timestamp(time),
        }
    }
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

impl ScrollWheelEvent {
    pub(crate) const fn new(event: &PointerEvent, time: u32, horizontal: AxisScroll, vertical: AxisScroll) -> Self {
        Self {
            scrolling_delta_x: LogicalPixels(horizontal.absolute),
            scrolling_delta_y: LogicalPixels(vertical.absolute),
            location_in_window: LogicalPoint {
                x: LogicalPixels(event.position.0),
                y: LogicalPixels(event.position.1),
            },
            timestamp: Timestamp(time),
        }
    }
}

impl From<ScrollWheelEvent> for Event<'_> {
    fn from(value: ScrollWheelEvent) -> Self {
        Self::ScrollWheel(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TextInputPreeditStringData<'a> {
    /// Can be null
    pub text: BorrowedStrPtr<'a>,
    pub cursor_begin_byte_pos: i32,
    pub cursor_end_byte_pos: i32,
}

impl Default for TextInputPreeditStringData<'_> {
    fn default() -> Self {
        Self {
            text: BorrowedStrPtr::new_optional(None),
            cursor_begin_byte_pos: 0,
            cursor_end_byte_pos: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct TextInputDeleteSurroundingTextData {
    pub before_length_in_bytes: u32,
    pub after_length_in_bytes: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct TextInputAvailabilityEvent {
    /// Indicates if the Text Input support is available.
    /// Call `application_text_input_enable` to enable it or `application_text_input_disable` to disable it afterward.
    pub available: bool,
}

/// The application must proceed by evaluating the changes in the following order:
/// 1. Replace the existing preedit string with the cursor.
/// 2. Delete the requested surrounding text.
/// 3. Insert the commit string with the cursor at its end.
/// 4. Calculate surrounding text to send.
/// 5. Insert the new preedit text in the cursor position.
/// 6. Place the cursor inside the preedit text.
#[repr(C)]
#[derive(Debug)]
pub struct TextInputEvent<'a> {
    pub has_preedit_string: bool,
    pub preedit_string: TextInputPreeditStringData<'a>,
    pub has_commit_string: bool,
    /// Can be null
    pub commit_string: BorrowedStrPtr<'a>,
    pub has_delete_surrounding_text: bool,
    pub delete_surrounding_text: TextInputDeleteSurroundingTextData,
}

impl<'a> From<TextInputEvent<'a>> for Event<'a> {
    fn from(value: TextInputEvent<'a>) -> Self {
        Self::TextInput(value)
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
pub struct WindowConfigureEvent {
    pub size: LogicalSize,
    pub active: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub client_side_decorations: bool,
    pub capabilities: WindowCapabilities,
}

impl From<WindowConfigureEvent> for Event<'_> {
    fn from(value: WindowConfigureEvent) -> Self {
        Self::WindowConfigure(value)
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
pub struct WindowFocusChangeEvent {
    pub is_key: bool,
    pub is_main: bool,
}

impl WindowFocusChangeEvent {
    pub(crate) const fn new(is_key: bool) -> Self {
        Self { is_key, is_main: is_key }
    }
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
pub struct WindowScreenChangeEvent {
    pub new_screen_id: ScreenId,
}

impl WindowScreenChangeEvent {
    pub(crate) fn new(output: &WlOutput) -> Self {
        Self {
            new_screen_id: ScreenId(output.id().protocol_id()),
        }
    }
}

impl From<WindowScreenChangeEvent> for Event<'_> {
    fn from(value: WindowScreenChangeEvent) -> Self {
        Self::WindowScreenChange(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum Event<'a> {
    DataTransfer(DataTransferContent<'a>),
    KeyDown(KeyDownEvent<'a>),
    KeyUp(KeyUpEvent<'a>),
    ModifiersChanged(ModifiersChangedEvent),
    MouseEntered(MouseEnteredEvent),
    MouseExited(MouseExitedEvent),
    MouseMoved(MouseMovedEvent),
    MouseDragged(MouseDraggedEvent),
    MouseDown(MouseDownEvent),
    MouseUp(MouseUpEvent),
    ScrollWheel(ScrollWheelEvent),
    TextInputAvailability(TextInputAvailabilityEvent),
    TextInput(TextInputEvent<'a>),
    WindowCloseRequest,
    WindowConfigure(WindowConfigureEvent),
    WindowDraw(WindowDrawEvent),
    WindowFocusChange(WindowFocusChangeEvent),
    WindowFullScreenToggle(WindowFullScreenToggleEvent),
    WindowScaleChanged(WindowScaleChangedEvent),
    WindowScreenChange(WindowScreenChangeEvent),
}
