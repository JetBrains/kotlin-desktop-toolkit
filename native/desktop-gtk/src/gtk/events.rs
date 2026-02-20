use core::f64;
use std::ffi::{CStr, CString};
use std::fmt::Write;

use desktop_common::ffi_utils::{BorrowedArray, BorrowedStrPtr};
use desktop_common::logger::PanicDefault;
use enumflags2::{BitFlag, BitFlags, bitflags};

use crate::gtk::{
    application_api::{DataSource, DragAndDropAction},
    geometry::{LogicalPixels, LogicalPoint, LogicalSize, PhysicalSize},
    xdg_desktop_settings_api::XdgDesktopSetting,
};

pub type EventHandler = extern "C" fn(&Event);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct Timestamp(pub u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenId(pub u64);

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

impl From<BitFlags<KeyModifier>> for KeyModifierBitflag {
    fn from(value: BitFlags<KeyModifier>) -> Self {
        Self(value.bits_c())
    }
}

impl KeyModifierBitflag {
    pub const EMPTY: Self = Self(0);
}

/// Raw XKB keycode
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
/// Some examples:
///
/// * `{ mime_type: "text/uri-list", data: "file:///data/some-file\r\nfile:///data/Some%20File%20With%20Spaces.txt\r\n" }`
/// * `{ mime_type: "text/plain;charset=utf-8", data: "some text\r\nhere" }`
pub struct DataTransferContent<'a> {
    pub mime_type: BorrowedStrPtr<'a>,
    pub data: BorrowedArray<'a, u8>,
}

impl<'a> DataTransferContent<'a> {
    #[must_use]
    pub fn new(mime_type: &'a CStr, data: &'a [u8]) -> Self {
        Self {
            mime_type: BorrowedStrPtr::new(mime_type),
            data: BorrowedArray::from_slice(data),
        }
    }

    #[must_use]
    pub fn null() -> Self {
        Self {
            mime_type: BorrowedStrPtr::null(),
            data: BorrowedArray::null(),
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
pub struct DragAndDropFinishedEvent {
    pub window_id: WindowId,
    pub action: DragAndDropAction,
}

impl From<DragAndDropFinishedEvent> for Event<'_> {
    fn from(value: DragAndDropFinishedEvent) -> Self {
        Self::DragAndDropFinished(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DropPerformedEvent<'a> {
    pub window_id: WindowId,
    pub content: DataTransferContent<'a>,
    pub action: DragAndDropAction,
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
pub struct KeyDownEvent {
    pub window_id: WindowId,
    pub code: KeyCode,
    pub has_character: bool,
    pub character: char,
    pub key: u32,
    // pub key_without_modifiers: u32,
    pub modifiers: KeyModifierBitflag,
    pub is_repeat: bool,
}

impl From<KeyDownEvent> for Event<'_> {
    fn from(value: KeyDownEvent) -> Self {
        Self::KeyDown(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent {
    pub window_id: WindowId,
    pub code: KeyCode,
    pub key: u32,
    // pub key_without_modifiers: u32,
}

impl From<KeyUpEvent> for Event<'_> {
    fn from(value: KeyUpEvent) -> Self {
        Self::KeyUp(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ModifiersChangedEvent {
    pub window_id: WindowId,
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
pub struct ScrollWheelEvent {
    pub window_id: WindowId,
    pub timestamp: Timestamp,
    pub scroll_delta_x: LogicalPixels,
    pub scroll_delta_y: LogicalPixels,
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
            text: BorrowedStrPtr::null(),
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
    pub window_id: WindowId,
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
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl WindowCapabilities {
    #[must_use]
    pub const fn all() -> Self {
        Self {
            window_menu: true,
            maximize: true,
            fullscreen: true,
            minimize: true,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowClosedEvent {
    pub window_id: WindowId,
}

impl From<WindowClosedEvent> for Event<'_> {
    fn from(value: WindowClosedEvent) -> Self {
        Self::WindowClosed(value)
    }
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowConfigureEvent {
    pub window_id: WindowId,
    pub size: LogicalSize,
    pub active: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub decoration_mode: WindowDecorationMode,
    pub capabilities: WindowCapabilities,
}

impl TryFrom<WindowConfigureEvent> for Event<'_> {
    type Error = ();

    fn try_from(value: WindowConfigureEvent) -> Result<Self, Self::Error> {
        if value.size.width == 0 || value.size.height == 0 {
            Err(())
        } else {
            Ok(Self::WindowConfigure(value))
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct ShouldRedraw {
    pub window_id: WindowId,
}

impl From<ShouldRedraw> for Event<'_> {
    fn from(value: ShouldRedraw) -> Self {
        Self::ShouldRedraw(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct OpenGlDrawData {
    pub framebuffer: u32,
    pub is_es: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowDrawEvent {
    pub window_id: WindowId,
    pub opengl_draw_data: OpenGlDrawData,
    pub physical_size: PhysicalSize,
}

impl From<WindowDrawEvent> for Event<'_> {
    fn from(value: WindowDrawEvent) -> Self {
        Self::WindowDraw(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct DragIconDrawEvent {
    pub opengl_draw_data: OpenGlDrawData,
    pub physical_size: PhysicalSize,
    pub scale: f64,
}

impl From<DragIconDrawEvent> for Event<'_> {
    fn from(value: DragIconDrawEvent) -> Self {
        Self::DragIconDraw(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowKeyboardEnterEvent {
    pub window_id: WindowId,
}

impl From<WindowKeyboardEnterEvent> for Event<'_> {
    fn from(value: WindowKeyboardEnterEvent) -> Self {
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
pub struct NotificationShownEvent {
    pub request_id: RequestId,

    /// Value `0` indicates an error.
    pub notification_id: u32,
}

impl From<NotificationShownEvent> for Event<'_> {
    fn from(value: NotificationShownEvent) -> Self {
        Self::NotificationShown(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct NotificationClosedEvent<'a> {
    pub notification_id: u32,

    /// Optional. Present only if notification was activated. By default, it has a value `"default"`.
    pub action: BorrowedStrPtr<'a>,

    /// Optional. Present only if notification was activated, and the application has an associated `.desktop` file.
    pub activation_token: BorrowedStrPtr<'a>,
}

impl<'a> NotificationClosedEvent<'a> {
    #[must_use]
    pub fn new(notification_id: u32, action: Option<&'a CString>, activation_token: Option<&'a CString>) -> Self {
        Self {
            notification_id,
            action: BorrowedStrPtr::new_optional(action),
            activation_token: BorrowedStrPtr::new_optional(activation_token),
        }
    }
}

impl<'a> From<NotificationClosedEvent<'a>> for Event<'a> {
    fn from(value: NotificationClosedEvent<'a>) -> Self {
        Self::NotificationClosed(value)
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum Event<'a> {
    ApplicationStarted,

    DisplayConfigurationChange,

    XdgDesktopSettingChange(XdgDesktopSetting<'a>),

    /// Data received from clipboard or primary selection. For drag&drop, see `DropPerformed`.
    DataTransfer(DataTransferEvent<'a>),

    /// Drag&drop targeting our application left the specified window.
    DragAndDropLeave(DragAndDropLeaveEvent),

    /// Drag&drop that was initiated from our window has finished.
    DragAndDropFinished(DragAndDropFinishedEvent),

    DragIconDraw(DragIconDrawEvent),

    /// Drag&drop targeting our window is finished, and we received data from it.
    DropPerformed(DropPerformedEvent<'a>),

    /// Reported for clipboard and primary selection.
    DataTransferAvailable(DataTransferAvailableEvent<'a>),

    /// Data transfer for data from our application was canceled
    DataTransferCancelled(DataTransferCancelledEvent),

    FileChooserResponse(FileChooserResponse<'a>),

    NotificationShown(NotificationShownEvent),
    NotificationClosed(NotificationClosedEvent<'a>),

    /// Modifier keys (e.g., Ctrl, Shift, etc.) are never reported. Use `ModifiersChanged` for them.
    KeyDown(KeyDownEvent),

    /// Modifier keys (e.g., Ctrl, Shift, etc.) are never reported. Use `ModifiersChanged` for them.
    KeyUp(KeyUpEvent),

    ModifiersChanged(ModifiersChangedEvent),
    MouseEntered(MouseEnteredEvent),
    MouseExited(MouseExitedEvent),
    MouseMoved(MouseMovedEvent),
    MouseDown(MouseDownEvent),
    MouseUp(MouseUpEvent),
    ShouldRedraw(ShouldRedraw),
    ShouldRedrawDragIcon,
    ScrollWheel(ScrollWheelEvent),
    TextInput(TextInputEvent<'a>),
    WindowClosed(WindowClosedEvent),
    WindowConfigure(WindowConfigureEvent),
    WindowDraw(WindowDrawEvent),
    WindowKeyboardEnter(WindowKeyboardEnterEvent),
    WindowKeyboardLeave(WindowKeyboardLeaveEvent),
    WindowScaleChanged(WindowScaleChangedEvent),
    WindowScreenChange(WindowScreenChangeEvent),
}
