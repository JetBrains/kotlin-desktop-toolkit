use std::{ffi::CString, str::FromStr};

use log::debug;
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        Connection, Proxy, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
};

use super::{
    application_state::ApplicationState,
    events::{Event, KeyDownEvent},
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
        if let Some(window) = self.get_window_mut(surface) {
            window.keyboard_enter(keysyms);
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, surface: &WlSurface, _serial: u32) {
        if let Some(window) = self.get_window_mut(surface) {
            window.keyboard_leave();
        }
        self.key_surface = None;
    }

    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, _serial: u32, event: KeyEvent) {
        if let Some(window) = self.get_key_window() {
            window.press_key(&event);
            //if let Some(keyboard_data) = keyboard.data::<KeyboardData<Self>>() {
            //    let seat = keyboard_data.seat();
            //    window.frame_action(seat, serial, frame_action);
            //}
        }
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, _serial: u32, event: KeyEvent) {
        if let Some(window) = self.get_key_window() {
            window.release_key(&event);
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
    pub fn keyboard_enter(&mut self, keysyms: &[Keysym]) {
        debug!("Keyboard focus on window with pressed syms: {keysyms:?}");
        self.keyboard_focus = true;
        (self.event_handler)(&Event::new_window_focus_change_event(true));
    }

    pub fn keyboard_leave(&mut self) {
        self.keyboard_focus = false;
        (self.event_handler)(&Event::new_window_focus_change_event(false));
    }

    pub fn press_key(&self, event: &KeyEvent) {
        let characters = event.utf8.as_ref().map(|s| CString::from_str(s).unwrap());
        let key = event.keysym.name().map(|s| CString::from_str(s).unwrap());
        let e = KeyDownEvent::new(event, characters.as_ref(), key.as_ref());
        (self.event_handler)(&(&e).into());
    }

    pub fn release_key(&self, event: &KeyEvent) {
        let characters = event.utf8.as_ref().map(|s| CString::from_str(s).unwrap());
        let key = event.keysym.name().map(|s| CString::from_str(s).unwrap());
        (self.event_handler)(&Event::new_key_up_event(event, characters.as_ref(), key.as_ref()));
    }

    pub fn update_modifiers(&self, modifiers: Modifiers) {
        (self.event_handler)(&Event::new_modifiers_changed_event(modifiers));
    }
}
