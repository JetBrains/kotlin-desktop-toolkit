use log::debug;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym, Modifiers};

use crate::linux::cursors::CURSORS;

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
        if event.keysym == Keysym::N {
            // Cycle through cursor icons.
            self.window_cursor_icon_idx = (self.window_cursor_icon_idx + 1) % CURSORS.len();
            debug!("Setting cursor icon to: {}", CURSORS[self.window_cursor_icon_idx].name());
            self.set_cursor = true;
        }
    }

    pub fn release_key(&mut self, _event: &KeyEvent) {}

    pub fn update_modifiers(&mut self, _: Modifiers, _layout: u32) {}
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct KeyModifiersSet(pub usize);

impl std::fmt::Debug for KeyModifiersSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KeyModifiersSet({:032b})", self.0)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct KeyCode(pub u16);
