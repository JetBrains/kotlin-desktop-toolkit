use smithay_client_toolkit::seat::keyboard::{KeyEvent, Keysym, Modifiers};

use crate::linux::cursors::CURSORS;

use super::window::SimpleWindow;

impl SimpleWindow {
    pub fn keyboard_enter(&mut self, keysyms: &[Keysym]) {
        println!("Keyboard focus on window with pressed syms: {keysyms:?}");
        self.keyboard_focus = true;
    }

    pub fn keyboard_leave(&mut self) {
        self.keyboard_focus = false;
    }

    pub fn press_key(&mut self, event: KeyEvent) {
        if event.keysym == Keysym::N {
            // Cycle through cursor icons.
            self.window_cursor_icon_idx = (self.window_cursor_icon_idx + 1) % CURSORS.len();
            println!("Setting cursor icon to: {}", CURSORS[self.window_cursor_icon_idx].name());
            self.set_cursor = true;
        }
    }

    pub fn release_key(&mut self, _: u32, _: KeyEvent) {}

    pub fn update_modifiers(&mut self, _: Modifiers, _layout: u32) {}
}
