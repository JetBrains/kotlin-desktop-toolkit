use std::ffi::CString;

use anyhow::Context;
use enumflags2::BitFlags;
use log::debug;
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
};

use super::events::{KeyUpEvent, ModifiersChangedEvent};
use crate::linux::{
    application_state::ApplicationState,
    events::{KeyCode, KeyDownEvent, KeyModifier, KeyModifierBitflag, WindowKeyboardEnterEvent, WindowKeyboardLeaveEvent},
};

pub fn send_key_down_event(state: &ApplicationState, event: KeyEvent, is_repeat: bool) {
    let characters = event.utf8.map(|s| {
        CString::new(s)
            .with_context(|| format!("event: raw_code={}, keysym={:?}", event.raw_code, event.keysym))
            .unwrap()
    });
    let code = KeyCode(event.raw_code);
    state.send_event(KeyDownEvent::new(code, event.keysym.raw(), characters.as_ref(), is_repeat));
}

impl KeyboardHandler for ApplicationState {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        surface: &WlSurface,
        _serial: u32,
        raw: &[u32],
        keysyms: &[Keysym],
    ) {
        if let Some(window) = self.get_window(surface) {
            debug!("Keyboard focus on window with pressed syms: {keysyms:?}");
            let ks: Vec<u32> = keysyms.iter().map(|e| e.raw()).collect();
            self.send_event(WindowKeyboardEnterEvent::new(window.window_id, raw, &ks));
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, surface: &WlSurface, _serial: u32) {
        if let Some(window) = self.get_window(surface) {
            self.send_event(WindowKeyboardLeaveEvent {
                window_id: window.window_id,
            });
        }
    }

    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, serial: u32, event: KeyEvent) {
        self.last_key_down_serial = Some(serial);
        send_key_down_event(self, event, false);
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, _serial: u32, event: KeyEvent) {
        debug!("KeyboardHandler::release_key");
        self.send_event(KeyUpEvent {
            code: KeyCode(event.raw_code),
            key: event.keysym.raw(),
        });
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
        let event = {
            let mut key_modifiers = BitFlags::<KeyModifier>::EMPTY;
            key_modifiers.set(KeyModifier::Ctrl, modifiers.ctrl);
            key_modifiers.set(KeyModifier::Alt, modifiers.alt);
            key_modifiers.set(KeyModifier::Shift, modifiers.shift);
            key_modifiers.set(KeyModifier::CapsLock, modifiers.caps_lock);
            key_modifiers.set(KeyModifier::Logo, modifiers.logo);
            key_modifiers.set(KeyModifier::NumLock, modifiers.num_lock);
            ModifiersChangedEvent {
                modifiers: KeyModifierBitflag(key_modifiers.bits_c()),
            }
        };
        self.send_event(event);
    }

    fn repeat_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, _serial: u32, event: KeyEvent) {
        send_key_down_event(self, event, true);
    }
}

delegate_keyboard!(ApplicationState);
