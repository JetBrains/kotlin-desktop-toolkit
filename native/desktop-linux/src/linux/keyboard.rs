use std::ffi::CString;

use enumflags2::BitFlags;
use log::{debug, info};
use smithay_client_toolkit::{
    delegate_keyboard,
    reexports::client::{
        Connection, QueueHandle,
        protocol::{wl_keyboard::WlKeyboard, wl_surface::WlSurface},
    },
    seat::keyboard::{KeyEvent, KeyboardHandler, Keymap, Keysym, Modifiers, RawModifiers},
};
use xkbcommon::xkb;

use super::events::{KeyUpEvent, ModifiersChangedEvent};
use crate::linux::{
    application_state::ApplicationState,
    events::{KeyCode, KeyDownEvent, KeyModifier, KeyModifierBitflag, WindowKeyboardEnterEvent, WindowKeyboardLeaveEvent},
};

pub fn send_key_down_event(state: &ApplicationState, event: KeyEvent, is_repeat: bool) {
    debug!("send_key_down_event: {event:?}, is_repeat={is_repeat}");
    let characters = event.utf8.and_then(|s| CString::new(s).ok());
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
        layout: u32,
    ) {
        debug!("KeyboardHandler::update_modifiers: layout={layout}, modifiers={modifiers:?}");
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

    fn update_keymap(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, keymap: Keymap<'_>) {
        let keymap_str = keymap.as_string();
        let keymap_len = keymap_str.len();
        debug!("KeyboardHandler::update_keymap: keymap_len={keymap_len}");
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let xkb_keymap =
            xkb::Keymap::new_from_string(&context, keymap_str, xkb::KEYMAP_FORMAT_TEXT_V1, xkb::KEYMAP_COMPILE_NO_FLAGS).unwrap();

        let ctrl_index = xkb_keymap.mod_get_index(xkb::MOD_NAME_CTRL);
        let mut xkb_state_ctrl_pressed = xkb::State::new(&xkb_keymap);
        xkb_state_ctrl_pressed.update_mask(1 << ctrl_index, 0, 0, 0, 0, 0);

        let alt_index = xkb_keymap.mod_get_index(xkb::MOD_NAME_ALT);
        let mut xkb_state_alt_pressed = xkb::State::new(&xkb_keymap);
        xkb_state_alt_pressed.update_mask(1 << alt_index, 0, 0, 0, 0, 0);

        for layout in 0..xkb_keymap.num_layouts() {
            let layout_name = xkb_keymap.layout_get_name(layout);
            debug!("KeyboardHandler::update_keymap: layout {layout} = {layout_name}");
            xkb_keymap.key_for_each(|keymap, keycode| {
                for level in 0..keymap.num_levels_for_key(keycode, layout) {
                    for keysym in keymap.key_get_syms_by_level(keycode, layout, level) {
                        let char_u32 = xkb::keysym_to_utf32(*keysym);
                        let utf8 = xkb::keysym_to_utf8(*keysym);
                        debug!("layout={layout}, level={level}, keycode={keycode:?}, keysym={keysym:?}, char_u32={char_u32}, utf8={utf8:?}");
                    }
                }
                let ctrl_key_u32 = xkb_state_ctrl_pressed.key_get_utf32(keycode);
                let ctrl_key_utf8 = xkb_state_ctrl_pressed.key_get_utf8(keycode);
                debug!("with CTRL: layout={layout}, keycode={keycode:?}, key_u32={ctrl_key_u32}, key_utf8={ctrl_key_utf8:?}");

                let alt_key_u32 = xkb_state_alt_pressed.key_get_utf32(keycode);
                let alt_key_utf8 = xkb_state_alt_pressed.key_get_utf8(keycode);
                debug!("with ALT: layout={layout}, keycode={keycode:?}, key_u32={alt_key_u32}, key_utf8={alt_key_utf8:?}");
            });
        }
    }
}

delegate_keyboard!(ApplicationState);
