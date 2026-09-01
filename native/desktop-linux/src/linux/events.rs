use crate::linux::{
    application_api::{DataSource, DragAndDropAction},
    desktop_settings_api::FfiDesktopSetting,
    geometry::{LogicalPixels, LogicalPixelsInt, LogicalPoint, LogicalSize, PhysicalSize, Scale},
};
use bitflag_attr::bitflag;
use desktop_common::ffi_utils::BorrowedUtf8;
use desktop_common::{ffi_utils::BorrowedArray, logger::PanicDefault};
use std::ffi::c_int;

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct EventSerial(pub(crate) u32);

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct EventSeat(pub(crate) u32);

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

#[repr(C)]
#[bitflag(c_int)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum KeyModifiers {
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

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct KeyCode(pub u32);

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowDecorationMode {
    /// The window should draw client side decorations.
    Client { frame: WindowFrame, tiling: WindowFrameTiling },

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
    pub mime_type: BorrowedUtf8<'a>,
    pub data: BorrowedArray<'a, u8>,
}

impl<'a> DataTransferContent<'a> {
    #[must_use]
    pub const fn new(mime_type: &'a str, data: &'a [u8]) -> Self {
        Self {
            mime_type: BorrowedUtf8::new(mime_type),
            data: BorrowedArray::from_slice(data),
        }
    }

    #[must_use]
    pub const fn null() -> Self {
        Self {
            mime_type: BorrowedUtf8::null(),
            data: BorrowedArray::null(),
        }
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
pub struct TextInputPreeditStringData<'a> {
    /// Can be null
    pub text: BorrowedUtf8<'a>,
    pub cursor_begin_byte_pos: i32,
    pub cursor_end_byte_pos: i32,
}

impl Default for TextInputPreeditStringData<'_> {
    fn default() -> Self {
        Self {
            text: BorrowedUtf8::null(),
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
#[derive(Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowFramePadding {
    pub left: LogicalPixelsInt,
    pub top: LogicalPixelsInt,
    pub right: LogicalPixelsInt,
    pub bottom: LogicalPixelsInt,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowFrameResizerThickness {
    pub left: LogicalPixelsInt,
    pub top: LogicalPixelsInt,
    pub right: LogicalPixelsInt,
    pub bottom: LogicalPixelsInt,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowFrameTiling {
    pub left: bool,
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WindowFrame {
    pub padding: WindowFramePadding,
    pub resizer_thickness: WindowFrameResizerThickness,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct WindowConfigureData {
    pub window_id: WindowId,
    pub size: LogicalSize,
    pub active: bool,
    pub maximized: bool,
    pub fullscreen: bool,
    pub decoration_mode: WindowDecorationMode,
    pub capabilities: WindowCapabilities,
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
}

#[repr(C)]
#[derive(Debug)]
pub struct DragIconDrawEvent {
    pub software_draw_data: SoftwareDrawData,
    pub physical_size: PhysicalSize,
    pub scale: Scale,
}

#[repr(C)]
#[derive(Debug)]
pub enum Event<'a> {
    ApplicationStarted,

    /// Return `true` from the event handler if the application should _not_ terminate.
    ApplicationWantsToTerminate,

    ApplicationWillTerminate,

    DisplayConfigurationChange,

    DesktopSettingChange(FfiDesktopSetting<'a>),

    /// Data received from clipboard or primary selection. For drag&drop, see `DropPerformed`.
    DataTransfer {
        serial: i32,
        content: DataTransferContent<'a>,
    },

    /// Drag&drop targeting our application left the specified window.
    DragAndDropLeave {
        window_id: WindowId,
    },

    /// Drag&drop that was initiated from our window has finished.
    DragAndDropFinished {
        window_id: WindowId,
        action: DragAndDropAction,
    },

    DragIconDraw(DragIconDrawEvent),

    /// Drag&drop targeting our window is finished, and we received data from it.
    DropPerformed {
        window_id: WindowId,
        content: DataTransferContent<'a>,
        action: DragAndDropAction,
        location_in_window: LogicalPoint,
    },

    /// Reported for clipboard and primary selection.
    DataTransferAvailable {
        data_source: DataSource,
        mime_types: BorrowedUtf8<'a>,
    },

    /// Data transfer for data from our application was canceled
    DataTransferCancelled {
        data_source: DataSource,
    },

    FileChooserResponse {
        request_id: RequestId,
        newline_separated_files: BorrowedUtf8<'a>,
    },

    ActivationTokenResponse {
        request_id: RequestId,
        token: BorrowedUtf8<'a>,
    },

    NotificationShown {
        request_id: RequestId,

        /// Value `0` indicates an error.
        notification_id: u32,
    },

    NotificationClosed {
        notification_id: u32,

        /// Optional. Present only if notification was activated. By default, it has a value `"default"`.
        action: BorrowedUtf8<'a>,

        /// Optional. Present only if notification was activated, and the application has an associated `.desktop` file.
        activation_token: BorrowedUtf8<'a>,
    },

    /// Modifier keys (e.g Ctrl, Shift, etc) are never reported. Use `ModifiersChanged` for them.
    KeyDown {
        serial: EventSerial,
        characters: BorrowedUtf8<'a>,
        code: KeyCode,
        key: u32,
        is_repeat: bool,
    },

    /// Modifier keys (e.g Ctrl, Shift, etc) are never reported. Use `ModifiersChanged` for them.
    KeyUp {
        serial: EventSerial,
        code: KeyCode,
        key: u32,
    },

    ModifiersChanged {
        serial: EventSerial,
        modifiers: KeyModifiers,
    },
    MouseEntered {
        serial: EventSerial,
        window_id: WindowId,
        location_in_window: LogicalPoint,
    },
    MouseExited {
        serial: EventSerial,
        window_id: WindowId,
        location_in_window: LogicalPoint,
    },
    MouseMoved {
        window_id: WindowId,
        location_in_window: LogicalPoint,
        timestamp: Timestamp,
    },
    MouseDown {
        serial: EventSerial,
        window_id: WindowId,
        button: MouseButton,
        location_in_window: LogicalPoint,
        timestamp: Timestamp,
    },
    MouseUp {
        serial: EventSerial,
        window_id: WindowId,
        button: MouseButton,
        location_in_window: LogicalPoint,
        timestamp: Timestamp,
    },
    ScrollWheel {
        window_id: WindowId,
        location_in_window: LogicalPoint,
        timestamp: Timestamp,
        horizontal_scroll: ScrollData,
        vertical_scroll: ScrollData,
    },
    TextInputAvailability {
        window_id: WindowId,

        /// Indicates if the Text Input support is available.
        /// Call `application_text_input_enable` to enable it or `application_text_input_disable` to disable it afterward.
        available: bool,
    },

    /// The application must proceed by evaluating the changes in the following order:
    /// 1. Replace the existing preedit string with the cursor.
    /// 2. Delete the requested surrounding text.
    /// 3. Insert the commit string with the cursor at its end.
    /// 4. Calculate surrounding text to send.
    /// 5. Insert the new preedit text in the cursor position.
    /// 6. Place the cursor inside the preedit text.
    TextInput {
        has_preedit_string: bool,
        preedit_string: TextInputPreeditStringData<'a>,
        has_commit_string: bool,
        /// Can be null
        commit_string: BorrowedUtf8<'a>,
        has_delete_surrounding_text: bool,
        delete_surrounding_text: TextInputDeleteSurroundingTextData,
    },

    WindowCloseRequest {
        window_id: WindowId,
    },
    WindowClosed {
        window_id: WindowId,
    },
    WindowConfigure(WindowConfigureData),
    WindowDraw(WindowDrawEvent),
    WindowKeyboardEnter {
        serial: EventSerial,
        window_id: WindowId,
        raw: BorrowedArray<'a, u32>,
        keysyms: BorrowedArray<'a, u32>,
    },
    WindowKeyboardLeave {
        serial: EventSerial,
        window_id: WindowId,
    },
    WindowScaleChanged {
        window_id: WindowId,
        new_scale: Scale,
    },
    WindowScreenChange {
        window_id: WindowId,
        new_screen_id: ScreenId,
    },
}
