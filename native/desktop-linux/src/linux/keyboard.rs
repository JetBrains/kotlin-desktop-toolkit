use crate::linux::{
    application_state::ApplicationState,
    events::{Event, EventSerial, KeyCode, KeyModifiers},
};
use desktop_common::ffi_utils::{BorrowedArray, BorrowedUtf8};
use log::debug;
use smithay_client_toolkit::{
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
};

pub fn send_key_down_event(state: &ApplicationState, event: &KeyEvent, serial: EventSerial, is_repeat: bool) {
    let code = KeyCode(event.raw_code + 8);
    let key = event.keysym.raw();
    let characters = event.utf8.as_ref();
    state.send_event(&Event::KeyDown {
        serial,
        code,
        characters: BorrowedUtf8::optional(characters.filter(|&s| !s.is_empty())),
        key,
        is_repeat,
    });
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
            let xkb_codes = raw.iter().map(|v| v + 8).collect::<Box<_>>();
            let ks: Vec<u32> = keysyms.iter().map(|e| e.raw()).collect();
            self.send_event(&Event::WindowKeyboardEnter {
                serial: EventSerial(serial),
                window_id,
                raw: BorrowedArray::from_slice(&xkb_codes),
                keysyms: BorrowedArray::from_slice(&ks),
            });
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, surface: &WlSurface, serial: u32) {
        if let Some(window_id) = self.get_window_id(surface) {
            self.send_event(&Event::WindowKeyboardLeave {
                serial: EventSerial(serial),
                window_id,
            });
        }
    }

    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, serial: u32, event: KeyEvent) {
        self.last_keyboard_event_serial = Some(serial);
        send_key_down_event(self, &event, EventSerial(serial), false);
    }

    fn repeat_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, serial: u32, event: KeyEvent) {
        send_key_down_event(self, &event, EventSerial(serial), true);
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, serial: u32, event: KeyEvent) {
        self.last_keyboard_event_serial = Some(serial);
        debug!("KeyboardHandler::release_key");
        self.send_event(&Event::KeyUp {
            serial: EventSerial(serial),
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
            Event::ModifiersChanged {
                serial: EventSerial(serial),
                modifiers: key_modifiers,
            }
        };
        self.send_event(&event);
    }
}
