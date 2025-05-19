use std::ffi::CString;

use log::debug;
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        Connection, Proxy, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
};

use super::events::{KeyUpEvent, ModifiersChangedEvent};
use crate::linux::{
    application_state::ApplicationState,
    events::{KeyCode, KeyDownEvent, WindowFocusChangeEvent},
    window::SimpleWindow,
};

impl KeyboardHandler for ApplicationState {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
        _raw: &[u32],
        keysyms: &[Keysym],
    ) {
        self.key_surface = Some(surface.id());
        if let Some(window) = self.get_window(surface) {
            window.keyboard_enter(keysyms);
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, surface: &WlSurface, _serial: u32) {
        if let Some(window) = self.get_window(surface) {
            window.keyboard_leave();
        }
        self.key_surface = None;
    }

    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, serial: u32, event: KeyEvent) {
        self.last_key_down_serial = Some(serial);
        if let Some(window) = self.get_key_window() {
            window.press_key(event);
        }
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, _serial: u32, event: KeyEvent) {
        if let Some(window) = self.get_key_window() {
            window.release_key(event);
        }
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _layout: u32,
    ) {
        if let Some(window) = self.get_key_window() {
            window.update_modifiers(modifiers);
        }
    }
}

delegate_keyboard!(ApplicationState);

impl SimpleWindow {
    pub fn keyboard_enter(&self, keysyms: &[Keysym]) {
        debug!("Keyboard focus on window with pressed syms: {keysyms:?}");
        (self.event_handler)(&WindowFocusChangeEvent::new(true).into());
    }

    pub fn keyboard_leave(&self) {
        (self.event_handler)(&WindowFocusChangeEvent::new(false).into());
    }

    fn send_key_down_event(&self, event: KeyEvent, is_repeat: bool) {
        let characters = event.utf8.map(|s| CString::new(s).unwrap());
        let e = KeyDownEvent::new(KeyCode(event.raw_code), event.keysym.raw(), characters.as_ref(), is_repeat);
        (self.event_handler)(&e.into());
    }

    pub fn press_key(&self, event: KeyEvent) {
        self.send_key_down_event(event, false);
    }

    pub fn on_key_repeat(&self, event: KeyEvent) {
        self.send_key_down_event(event, true);
    }

    pub fn release_key(&self, event: KeyEvent) {
        let characters = event.utf8.map(|s| CString::new(s).unwrap());
        (self.event_handler)(&KeyUpEvent::new(KeyCode(event.raw_code), event.keysym.raw(), characters.as_ref()).into());
    }

    pub fn update_modifiers(&self, modifiers: Modifiers) {
        (self.event_handler)(&ModifiersChangedEvent::new(modifiers).into());
    }
}
