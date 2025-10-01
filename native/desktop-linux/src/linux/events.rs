use core::f64;
use std::{
    ffi::{CStr, CString},
    fmt::Write,
};

use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedStrPtr},
    logger::PanicDefault,
};
use enumflags2::{BitFlag, BitFlags, bitflags};

use crate::linux::{
    application_api::DataSource,
    geometry::{LogicalPixels, LogicalPoint, LogicalSize, PhysicalSize},
    xdg_desktop_settings_api::XdgDesktopSetting,
};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Timestamp(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct ScreenId(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct WindowId(pub i64);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct RequestId(pub u32);

impl PanicDefault for RequestId {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct MouseButton(pub u32);

#[bitflags]
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum KeyModifier {
    /// The "control" key
    Ctrl = 0b0000_0001,

    /// The "alt" key
    Alt = 0b0000_0010,

    /// The "shift" key
    Shift = 0b0000_0100,

    /// The "Caps lock" key
    CapsLock = 0b0000_1000,

    /// The "logo" key
    ///
    /// Also known as the "windows" or "super" key on a keyboard.
    Logo = 0b0001_0000,

    /// The "Num lock" key
    NumLock = 0b0010_0000,
}

#[derive(Default, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct KeyModifierBitflag(pub u8);

impl std::fmt::Debug for KeyModifierBitflag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeyModifierBitflag({:#08b}, ", &self.0)?;
        let bitflags = KeyModifier::from_bits(self.0).unwrap();

        for (i, field) in bitflags.into_iter().enumerate() {
            if i > 0 {
                f.write_char('|')?;
            }
            field.fmt(f)?;
        }

        f.write_char(')')
    }
}

impl From<KeyModifier> for KeyModifierBitflag {
    fn from(value: KeyModifier) -> Self {
        Self(BitFlags::from_flag(value).bits_c())
    }
}

impl From<BitFlags<KeyModifier>> for KeyModifierBitflag {
    fn from(value: BitFlags<KeyModifier>) -> Self {
        Self(value.bits_c())
    }
}

impl KeyModifierBitflag {
    pub const EMPTY: Self = Self(0);
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct KeyCode(pub u32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDecorationMode {
    /// The window should draw client side decorations.
    Client,

    /// The server will draw window decorations.
    Server,
}

#[repr(C)]
#[derive(Debug)]
pub struct DataTransferContent<'a> {
    pub data: BorrowedArray<'a, u8>,
    pub mime_types: BorrowedStrPtr<'a>,
}

impl<'a> DataTransferContent<'a> {
    #[must_use]
    pub fn new(data: &'a [u8], mime_types: &'a CStr) -> Self {
        Self {
            data: BorrowedArray::from_slice(data),
            mime_types: BorrowedStrPtr::new(mime_types),
        }
    }

    #[must_use]
    pub fn null() -> Self {
        Self {
            data: BorrowedArray::null(),
            mime_types: BorrowedStrPtr::null(),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DataTransferEvent<'a> {
    pub serial: i32,
    pub content: DataTransferContent<'a>,
}

impl<'a> From<DataTransferEvent<'a>> for Event<'a> {
    fn from(value: DataTransferEvent<'a>) -> Self {
        Self::DataTransfer(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DragAndDropLeaveEvent {
    pub window_id: WindowId,
}

impl From<DragAndDropLeaveEvent> for Event<'_> {
    fn from(value: DragAndDropLeaveEvent) -> Self {
        Self::DragAndDropLeave(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DropPerformedEvent<'a> {
    pub window_id: WindowId,
    pub content: DataTransferContent<'a>,
}

impl<'a> From<DropPerformedEvent<'a>> for Event<'a> {
    fn from(value: DropPerformedEvent<'a>) -> Self {
        Self::DropPerformed(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DataTransferAvailableEvent<'a> {
    pub data_source: DataSource,
    pub mime_types: BorrowedStrPtr<'a>,
}

impl<'a> From<DataTransferAvailableEvent<'a>> for Event<'a> {
    fn from(value: DataTransferAvailableEvent<'a>) -> Self {
        Self::DataTransferAvailable(value)
    }
}

impl<'a> DataTransferAvailableEvent<'a> {
    #[must_use]
    pub const fn new(data_source: DataSource, mime_types: &'a CStr) -> Self {
        Self {
            data_source,
            mime_types: BorrowedStrPtr::new(mime_types),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DataTransferCancelledEvent {
    pub data_source: DataSource,
}

impl From<DataTransferCancelledEvent> for Event<'_> {
    fn from(value: DataTransferCancelledEvent) -> Self {
        Self::DataTransferCancelled(value)
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
    pub(crate) fn new(code: KeyCode, key: u32, characters: Option<&'a CString>, is_repeat: bool) -> Self {
        Self {
            code,
            characters: BorrowedStrPtr::new_optional(characters),
            key,
            is_repeat,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent {
    pub code: KeyCode,
    pub key: u32,
}

impl From<KeyUpEvent> for Event<'_> {
    fn from(value: KeyUpEvent) -> Self {
        Self::KeyUp(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ModifiersChangedEvent {
    pub modifiers: KeyModifierBitflag,
}

impl From<ModifiersChangedEvent> for Event<'_> {
    fn from(value: ModifiersChangedEvent) -> Self {
        Self::ModifiersChanged(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseEnteredEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
}

impl From<MouseEnteredEvent> for Event<'_> {
    fn from(value: MouseEnteredEvent) -> Self {
        Self::MouseEntered(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseExitedEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
}

impl From<MouseExitedEvent> for Event<'_> {
    fn from(value: MouseExitedEvent) -> Self {
        Self::MouseExited(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    pub window_id: WindowId,
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
pub struct MouseDownEvent {
    pub window_id: WindowId,
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

impl From<MouseDownEvent> for Event<'_> {
    fn from(value: MouseDownEvent) -> Self {
        Self::MouseDown(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseUpEvent {
    pub window_id: WindowId,
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
pub struct ScrollData {
    pub delta: LogicalPixels,
    pub wheel_value120: i32,
    pub is_inverted: bool,
    pub is_stop: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
    pub horizontal_scroll: ScrollData,
    pub vertical_scroll: ScrollData,
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
    pub window_id: WindowId,
    /// Indicates if the Text Input support is available.
    /// Call `application_text_input_enable` to enable it or `application_text_input_disable` to disable it afterward.
    pub available: bool,
}

impl From<TextInputAvailabilityEvent> for Event<'_> {
    fn from(value: TextInputAvailabilityEvent) -> Self {
        Self::TextInputAvailability(value)
    }
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
    pub maximize: bool,

    /// Window can be fullscreened and unfullscreened.
    pub fullscreen: bool,

    /// Window can be minimized.
    pub minimize: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowCloseRequestEvent {
    pub window_id: WindowId,
}

impl From<WindowCloseRequestEvent> for Event<'_> {
    fn from(value: WindowCloseRequestEvent) -> Self {
        Self::WindowCloseRequest(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowConfigureEvent {
    pub window_id: WindowId,
    pub size: LogicalSize,
    pub active: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub decoration_mode: WindowDecorationMode,
    pub capabilities: WindowCapabilities,
}

impl From<WindowConfigureEvent> for Event<'_> {
    fn from(value: WindowConfigureEvent) -> Self {
        Self::WindowConfigure(value)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct SoftwareDrawData {
    /// Can be null, to indicate that the software drawing is not being used
    pub canvas: *mut u8,
    pub stride: i32,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowDrawEvent {
    pub window_id: WindowId,
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
pub struct WindowKeyboardEnterEvent<'a> {
    pub window_id: WindowId,
    pub raw: BorrowedArray<'a, u32>,
    pub keysyms: BorrowedArray<'a, u32>,
}

impl<'a> WindowKeyboardEnterEvent<'a> {
    pub(crate) fn new(window_id: WindowId, raw: &'a [u32], keysyms: &'a [u32]) -> Self {
        Self {
            window_id,
            raw: BorrowedArray::from_slice(raw),
            keysyms: BorrowedArray::from_slice(keysyms),
        }
    }
}

impl<'a> From<WindowKeyboardEnterEvent<'a>> for Event<'a> {
    fn from(value: WindowKeyboardEnterEvent<'a>) -> Self {
        Self::WindowKeyboardEnter(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowKeyboardLeaveEvent {
    pub window_id: WindowId,
}

impl From<WindowKeyboardLeaveEvent> for Event<'_> {
    fn from(value: WindowKeyboardLeaveEvent) -> Self {
        Self::WindowKeyboardLeave(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScaleChangedEvent {
    pub window_id: WindowId,
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
    pub window_id: WindowId,
    pub new_screen_id: ScreenId,
}

impl From<WindowScreenChangeEvent> for Event<'_> {
    fn from(value: WindowScreenChangeEvent) -> Self {
        Self::WindowScreenChange(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct FileChooserResponse<'a> {
    pub window_id: WindowId,
    pub request_id: RequestId,
    pub newline_separated_files: BorrowedStrPtr<'a>,
}

impl<'a> From<FileChooserResponse<'a>> for Event<'a> {
    fn from(value: FileChooserResponse<'a>) -> Self {
        Self::FileChooserResponse(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum Event<'a> {
    ApplicationStarted,

    /// Return `true` from the event handler if the application should _not_ terminate.
    ApplicationWantsToTerminate,

    ApplicationWillTerminate,

    DisplayConfigurationChange,

    XdgDesktopSettingChange(XdgDesktopSetting<'a>),

    /// Data received from clipboard or primary selection. For drag&drop, see `DropPerformed`.
    DataTransfer(DataTransferEvent<'a>),

    /// Drag&drop targeting our application left the specified window.
    DragAndDropLeave(DragAndDropLeaveEvent),

    /// Drag&drop targeting our window is finished, and we received data from it.
    DropPerformed(DropPerformedEvent<'a>),

    /// Reported for clipboard and primary selection.
    DataTransferAvailable(DataTransferAvailableEvent<'a>),

    /// Data transfer for data from our application was canceled
    DataTransferCancelled(DataTransferCancelledEvent),

    FileChooserResponse(FileChooserResponse<'a>),

    /// Modifier keys (e.g Ctrl, Shift, etc) are never reported. Use `ModifiersChanged` for them.
    KeyDown(KeyDownEvent<'a>),

    /// Modifier keys (e.g Ctrl, Shift, etc) are never reported. Use `ModifiersChanged` for them.
    KeyUp(KeyUpEvent),

    ModifiersChanged(ModifiersChangedEvent),
    MouseEntered(MouseEnteredEvent),
    MouseExited(MouseExitedEvent),
    MouseMoved(MouseMovedEvent),
    MouseDown(MouseDownEvent),
    MouseUp(MouseUpEvent),
    ScrollWheel(ScrollWheelEvent),
    TextInputAvailability(TextInputAvailabilityEvent),
    TextInput(TextInputEvent<'a>),
    WindowCloseRequest(WindowCloseRequestEvent),
    WindowConfigure(WindowConfigureEvent),
    WindowDraw(WindowDrawEvent),
    WindowKeyboardEnter(WindowKeyboardEnterEvent<'a>),
    WindowKeyboardLeave(WindowKeyboardLeaveEvent),
    WindowScaleChanged(WindowScaleChangedEvent),
    WindowScreenChange(WindowScreenChangeEvent),
}
