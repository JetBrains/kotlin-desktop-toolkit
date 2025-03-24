use std::{ffi::CString, str::FromStr};

use log::debug;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym, Modifiers};

use super::{events::Event, window::SimpleWindow};

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

    pub fn press_key(&mut self, event: &KeyEvent) {
        let characters = event.utf8.as_ref().map(|s| CString::from_str(s).unwrap());
        let key = event.keysym.name().map(|s| CString::from_str(s).unwrap());
        (self.event_handler)(&Event::new_key_down_event(event, characters.as_ref(), key.as_ref()));
    }

    pub fn release_key(&mut self, event: &KeyEvent) {
        let characters = event.utf8.as_ref().map(|s| CString::from_str(s).unwrap());
        let key = event.keysym.name().map(|s| CString::from_str(s).unwrap());
        (self.event_handler)(&Event::new_key_up_event(event, characters.as_ref(), key.as_ref()));
    }

    pub fn update_modifiers(&mut self, modifiers: Modifiers, _layout: u32) {
        (self.event_handler)(&Event::new_modifiers_changed_event(modifiers));
    }
}

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
