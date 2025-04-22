use std::{ffi::CString, str::FromStr};

use log::debug;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym, Modifiers};

use super::{
    events::{Event, KeyDownEvent},
    window::SimpleWindow,
};

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
