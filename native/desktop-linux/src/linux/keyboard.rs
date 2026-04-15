use log::debug;
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        Connection, Proxy as _, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardData, KeyboardHandler, Keysym, Modifiers, RawModifiers},
};

use crate::linux::{
    application_state::ApplicationState,
    events::{KeyCode, KeyDownEvent, KeyModifiers, KeyUpEvent, ModifiersChangedEvent, WindowKeyboardEnterEvent, WindowKeyboardLeaveEvent},
};

pub fn send_key_down_event(state: &ApplicationState, event: &KeyEvent, is_repeat: bool) {
    let code = KeyCode(event.raw_code + 8);
    state.send_event(KeyDownEvent::new(code, event.keysym.raw(), event.utf8.as_ref(), is_repeat));
}

impl KeyboardHandler for ApplicationState {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        surface: &WlSurface,
        serial: u32,
        raw: &[u32],
        keysyms: &[Keysym],
    ) {
        self.last_keyboard_event_serial = Some(serial);
        if let Some(window_id) = self.get_window_id(surface) {
            debug!("Keyboard focus on window with pressed syms: {keysyms:?}");
            let xkb_codes = raw.iter().map(|v| v + 8).collect();
            let ks: Vec<u32> = keysyms.iter().map(|e| e.raw()).collect();
            self.send_event(WindowKeyboardEnterEvent::new(window_id, &xkb_codes, &ks));
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, surface: &WlSurface, _serial: u32) {
        if let Some(window_id) = self.get_window_id(surface) {
            self.send_event(WindowKeyboardLeaveEvent { window_id });
        }
    }

    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, keyboard: &WlKeyboard, serial: u32, event: KeyEvent) {
        self.last_keyboard_event_serial = Some(serial);
        if let Some(keyboard_data) = keyboard.data::<KeyboardData<Self>>() {
            let seat = keyboard_data.seat();
            debug!("KeyboardHandler::press_key: setting last_implicit_grab_seat to {}", seat.id());
        }
        send_key_down_event(self, &event, false);
    }

    fn repeat_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, _serial: u32, event: KeyEvent) {
        send_key_down_event(self, &event, true);
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, serial: u32, event: KeyEvent) {
        self.last_keyboard_event_serial = Some(serial);
        debug!("KeyboardHandler::release_key");
        self.send_event(KeyUpEvent {
            code: KeyCode(event.raw_code + 8),
            key: event.keysym.raw(),
        });
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        self.last_keyboard_event_serial = Some(serial);
        let event = {
            let mut key_modifiers = KeyModifiers::empty();
            if modifiers.ctrl {
                key_modifiers |= KeyModifiers::Ctrl;
            }
            if modifiers.alt {
                key_modifiers |= KeyModifiers::Alt;
            }
            if modifiers.shift {
                key_modifiers |= KeyModifiers::Shift;
            }
            if modifiers.caps_lock {
                key_modifiers |= KeyModifiers::CapsLock;
            }
            if modifiers.logo {
                key_modifiers |= KeyModifiers::Logo;
            }
            if modifiers.num_lock {
                key_modifiers |= KeyModifiers::NumLock;
            }
            ModifiersChangedEvent { modifiers: key_modifiers }
        };
        self.send_event(event);
    }
}

delegate_keyboard!(ApplicationState);
