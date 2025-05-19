#![allow(clippy::let_and_return)]

use core::f64;

use anyhow::bail;
use log::warn;
use objc2_app_kit::{NSEvent, NSEventType, NSScreen, NSWindow};
use objc2_foundation::{MainThreadMarker, NSArray, NSURL};

use desktop_common::{
    ffi_utils::{AutoDropArray, BorrowedStrPtr, RustAllocatedStrPtr},
    logger::{PanicDefault, ffi_boundary},
};

use crate::geometry::{LogicalPixels, LogicalPoint, LogicalSize};

use super::{
    appearance::Appearance,
    application_api::AppState,
    keyboard::{EMPTY_KEY_MODIFIERS, KeyCode, KeyModifiersSet, unpack_flags_changed_event, unpack_key_event},
    mouse::{EmptyMouseButtonsSet, MouseButton, MouseButtonsSet, NSMouseEventExt},
    screen::{NSScreenExts, ScreenId},
    string::{borrow_ns_string, copy_to_c_string},
    url::url_to_absolute_string,
    window::NSWindowExts,
    window_api::WindowId,
};

// return true if event was handled
pub type EventHandler = extern "C" fn(&Event) -> bool;
pub type Timestamp = f64;

#[repr(C)]
#[derive(Debug)]
pub struct KeyDownEvent<'a> {
    pub window_id: WindowId,
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: BorrowedStrPtr<'a>,
    pub key_with_modifiers: BorrowedStrPtr<'a>,
    pub is_repeat: bool,
    pub might_have_key_equivalent: bool,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct KeyUpEvent<'a> {
    pub window_id: WindowId,
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub characters: BorrowedStrPtr<'a>,
    pub key: BorrowedStrPtr<'a>,
    pub key_with_modifiers: BorrowedStrPtr<'a>,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ModifiersChangedEvent {
    pub window_id: WindowId,
    pub modifiers: KeyModifiersSet,
    pub code: KeyCode,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseMovedEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDraggedEvent {
    pub window_id: WindowId,
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseEnteredEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseExitedEvent {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseDownEvent {
    pub window_id: WindowId,
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub click_count: isize,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct MouseUpEvent {
    pub window_id: WindowId,
    pub button: MouseButton,
    pub location_in_window: LogicalPoint,
    pub click_count: isize,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct ScrollWheelEvent {
    pub window_id: WindowId,
    pub scrolling_delta_x: LogicalPixels,
    pub scrolling_delta_y: LogicalPixels,
    pub has_precise_scrolling_deltas: bool,
    pub is_direction_inverted: bool,
    pub location_in_window: LogicalPoint,
    pub timestamp: Timestamp,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowScreenChangeEvent {
    pub window_id: WindowId,
    pub new_screen_id: ScreenId,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowResizeEvent {
    pub window_id: WindowId,
    pub size: LogicalSize,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowMoveEvent {
    pub window_id: WindowId,
    pub origin: LogicalPoint,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFocusChangeEvent {
    pub window_id: WindowId,
    pub is_key: bool,
    pub is_main: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowCloseRequestEvent {
    pub window_id: WindowId,
}

#[repr(C)]
#[derive(Debug)]
pub struct WindowFullScreenToggleEvent {
    pub window_id: WindowId,
    pub is_full_screen: bool,
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationOpenUrlsEvent {
    pub urls: AutoDropArray<RustAllocatedStrPtr>,
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationAppearanceChangeEvent {
    pub new_appearance: Appearance,
}

#[repr(C)]
#[derive(Debug)]
pub enum Event<'a> {
    KeyDown(KeyDownEvent<'a>),
    KeyUp(KeyUpEvent<'a>),
    ModifiersChanged(ModifiersChangedEvent),
    MouseMoved(MouseMovedEvent),
    MouseDragged(MouseDraggedEvent),
    MouseEntered(MouseEnteredEvent),
    MouseExited(MouseExitedEvent),
    MouseDown(MouseDownEvent),
    MouseUp(MouseUpEvent),
    ScrollWheel(ScrollWheelEvent),
    WindowScreenChange(WindowScreenChangeEvent),
    WindowResize(WindowResizeEvent),
    WindowMove(WindowMoveEvent),
    WindowFocusChange(WindowFocusChangeEvent),
    WindowCloseRequest(WindowCloseRequestEvent),
    WindowFullScreenToggle(WindowFullScreenToggleEvent),
    DisplayConfigurationChange,
    ApplicationOpenUrls(ApplicationOpenUrlsEvent),
    ApplicationDidFinishLaunching,
    ApplicationAppearanceChange(ApplicationAppearanceChangeEvent),
}

pub(crate) fn handle_key_down_event(ns_event: &NSEvent, might_have_key_equivalent: bool) -> anyhow::Result<bool> {
    let handled = AppState::with(|state| match unsafe { ns_event.r#type() } {
        NSEventType::KeyDown => {
            let key_info = unpack_key_event(ns_event)?;
            let event = Event::KeyDown(KeyDownEvent {
                window_id: ns_event.window_id(),
                code: key_info.code,
                is_repeat: key_info.is_repeat,
                characters: borrow_ns_string(&key_info.typed_chars),
                key: borrow_ns_string(&key_info.key),
                key_with_modifiers: borrow_ns_string(&key_info.key_with_modifiers),
                modifiers: key_info.modifiers,
                timestamp: unsafe { ns_event.timestamp() },
                might_have_key_equivalent,
            });
            Ok((state.event_handler)(&event))
        }
        _ => bail!("Unexpected type of event {:?}", ns_event),
    });
    handled
}

pub(crate) fn handle_key_up_event(ns_event: &NSEvent) -> anyhow::Result<bool> {
    let handled = AppState::with(|state| match unsafe { ns_event.r#type() } {
        NSEventType::KeyUp => {
            let key_info = unpack_key_event(ns_event)?;
            let event = Event::KeyUp(KeyUpEvent {
                window_id: ns_event.window_id(),
                code: key_info.code,
                characters: borrow_ns_string(&key_info.typed_chars),
                key: borrow_ns_string(&key_info.key),
                key_with_modifiers: borrow_ns_string(&key_info.key_with_modifiers),
                modifiers: key_info.modifiers,
                timestamp: unsafe { ns_event.timestamp() },
            });
            Ok((state.event_handler)(&event))
        }
        _ => bail!("Unexpected type of event {:?}", ns_event),
    });
    handled
}

pub(crate) fn handle_flags_change(ns_event: &NSEvent) -> anyhow::Result<bool> {
    let handled = AppState::with(|state| {
        let flags_changed_info = unpack_flags_changed_event(ns_event)?;
        let event = Event::ModifiersChanged(ModifiersChangedEvent {
            window_id: ns_event.window_id(),
            modifiers: flags_changed_info.modifiers,
            code: flags_changed_info.code,
            timestamp: unsafe { ns_event.timestamp() },
        });
        Ok((state.event_handler)(&event))
    });
    handled
}

pub(crate) fn handle_mouse_move(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseMoved(MouseMovedEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_drag(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseDragged(MouseDraggedEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_enter(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseEntered(MouseEnteredEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_exit(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseExited(MouseExitedEvent {
            window_id: ns_event.window_id(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_down(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseDown(MouseDownEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            click_count: unsafe { ns_event.clickCount() },
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_mouse_up(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::MouseUp(MouseUpEvent {
            window_id: ns_event.window_id(),
            button: ns_event.mouse_button().unwrap(),
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            click_count: unsafe { ns_event.clickCount() },
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_scroll_wheel(ns_event: &NSEvent) -> bool {
    let handled = AppState::with(|state| {
        let event = Event::ScrollWheel(ScrollWheelEvent {
            window_id: ns_event.window_id(),
            scrolling_delta_x: unsafe { ns_event.scrollingDeltaX() },
            scrolling_delta_y: unsafe { ns_event.scrollingDeltaY() },
            has_precise_scrolling_deltas: unsafe { ns_event.hasPreciseScrollingDeltas() },
            is_direction_inverted: unsafe { ns_event.isDirectionInvertedFromDevice() },
            location_in_window: ns_event.cursor_location_in_window(state.mtm),
            timestamp: unsafe { ns_event.timestamp() },
        });
        (state.event_handler)(&event)
    });
    handled
}

pub(crate) fn handle_window_screen_change(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowScreenChange(WindowScreenChangeEvent {
            window_id: window.window_id(),
            // todo sometimes it panics when you close the lid
            new_screen_id: window.screen().unwrap().screen_id(),
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_resize(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowResize(WindowResizeEvent {
            window_id: window.window_id(),
            size: window.get_size(),
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_move(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowMove(WindowMoveEvent {
            window_id: window.window_id(),
            origin: window.get_origin(state.mtm).unwrap(), // todo
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_close_request(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowCloseRequest(WindowCloseRequestEvent {
            window_id: window.window_id(),
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_focus_change(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowFocusChange(WindowFocusChangeEvent {
            window_id: window.window_id(),
            is_key: window.isKeyWindow(),
            is_main: unsafe { window.isMainWindow() },
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_window_full_screen_toggle(window: &NSWindow) {
    let _handled = AppState::with(|state| {
        let event = Event::WindowFullScreenToggle(WindowFullScreenToggleEvent {
            window_id: window.window_id(),
            is_full_screen: window.is_full_screen(),
        });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_display_configuration_change() {
    let _handled = AppState::with(|state| {
        let event = Event::DisplayConfigurationChange;
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_application_did_finish_launching() {
    let _handled = AppState::with(|state| {
        let event = Event::ApplicationDidFinishLaunching;
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_application_open_urls(urls: &NSArray<NSURL>) {
    let urls = urls
        .iter()
        .filter_map(|url| {
            let url_ns_string = url_to_absolute_string(&url);
            if url_ns_string.is_none() {
                warn!("Skipped the open url: {url:?}");
            }
            url_ns_string
        })
        .map(|url_ns_string| copy_to_c_string(&url_ns_string).unwrap())
        .collect::<Box<_>>();
    let urls = AutoDropArray::new(urls);
    let _handled = AppState::with(|state| {
        let event = Event::ApplicationOpenUrls(ApplicationOpenUrlsEvent { urls });
        (state.event_handler)(&event)
    });
}

pub(crate) fn handle_application_appearance_change() {
    let _handled = AppState::with(|state| {
        let new_appearance = Appearance::from_ns_appearance(&state.app.effectiveAppearance());
        let event = Event::ApplicationAppearanceChange(ApplicationAppearanceChangeEvent { new_appearance });
        (state.event_handler)(&event)
    });
}

impl PanicDefault for MouseButtonsSet {
    fn default() -> Self {
        EmptyMouseButtonsSet
    }
}

#[unsafe(no_mangle)]
extern "C" fn events_pressed_mouse_buttons() -> MouseButtonsSet {
    ffi_boundary("events_pressed_mouse_buttons", || Ok(NSEvent::pressed_mouse_buttons()))
}

impl PanicDefault for KeyModifiersSet {
    fn default() -> Self {
        EMPTY_KEY_MODIFIERS
    }
}

#[unsafe(no_mangle)]
extern "C" fn events_pressed_modifiers() -> KeyModifiersSet {
    ffi_boundary("events_pressed_modifiers", || Ok(NSEvent::pressed_modifiers()))
}

#[unsafe(no_mangle)]
extern "C" fn events_cursor_location_in_screen() -> LogicalPoint {
    ffi_boundary("events_cursor_location_in_screen", || {
        let mtm = MainThreadMarker::new().unwrap();
        Ok(NSEvent::cursor_location_in_screen(mtm))
    })
}

trait NSEventExt {
    fn me(&self) -> &NSEvent;

    fn window_id(&self) -> WindowId {
        let me = self.me();
        unsafe { me.windowNumber() }
    }

    fn cursor_location_in_window(&self, mtm: MainThreadMarker) -> LogicalPoint {
        let me = self.me();
        let point = unsafe {
            // position is relative to bottom left corner of the root view
            me.locationInWindow()
        };
        let window = unsafe { me.window(mtm).expect("No window for event") };
        let frame = window.contentView().unwrap().frame();
        LogicalPoint::from_macos_coords(point, frame.size.height)
    }

    fn cursor_location_in_screen(mtm: MainThreadMarker) -> LogicalPoint {
        let point = unsafe { NSEvent::mouseLocation() };
        let screen = NSScreen::primary(mtm).unwrap();
        LogicalPoint::from_macos_coords(point, screen.height())
    }

    fn pressed_modifiers() -> KeyModifiersSet {
        unsafe { NSEvent::modifierFlags_class() }.into()
    }
}

impl NSEventExt for NSEvent {
    fn me(&self) -> &NSEvent {
        self
    }
}
