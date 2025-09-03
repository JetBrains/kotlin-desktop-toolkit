use std::ffi::CString;

use log::debug;
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
    events::{KeyCode, KeyDownEvent, WindowKeyboardEnterEvent, WindowKeyboardLeaveEvent},
    virtual_keys::VirtualKey,
};

pub fn send_key_down_event(state: &ApplicationState, event: KeyEvent, is_repeat: bool, keymap_keys: &[KeymapKey], xkb_current_layout: u32) {
    debug!("send_key_down_event: {event:?}, raw keysym = {}", event.keysym.raw());
    let characters = event.utf8.map(|s| CString::new(s).unwrap());
    let code = KeyCode(event.raw_code);
    let vk = VirtualKey::new(event.keysym, code, keymap_keys, xkb_current_layout);
    state.send_event(KeyDownEvent::new(code, event.keysym.raw(), characters.as_ref(), is_repeat, vk));
}

#[derive(Debug)]
pub struct KeymapKey {
    pub keycode: KeyCode,
    pub keysym: Keysym,
    pub char_u32: u32,
    pub layout: u32,
    pub level: u32,
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
        debug!("KeyboardHandler::enter");
        if let Some(window) = self.get_window(surface) {
            debug!("Keyboard focus on window with pressed syms: {keysyms:?}");
            let ks: Vec<u32> = keysyms.iter().map(|e| e.raw()).collect();
            self.send_event(WindowKeyboardEnterEvent::new(window.window_id, raw, &ks));
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlKeyboard, surface: &WlSurface, _serial: u32) {
        debug!("KeyboardHandler::leave");
        if let Some(window) = self.get_window(surface) {
            self.send_event(WindowKeyboardLeaveEvent {
                window_id: window.window_id,
            });
        }
    }

    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, serial: u32, event: KeyEvent) {
        debug!("KeyboardHandler::press_key");
        self.last_key_down_serial = Some(serial);
        send_key_down_event(self, event, false, &self.keymap_keys, self.xkb_current_layout);
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
        if self.xkb_current_layout != layout {
            // TODO: add layout changed event
            self.xkb_current_layout = layout;
        }

        self.send_event(ModifiersChangedEvent::new(modifiers));
    }

    fn repeat_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, _serial: u32, event: KeyEvent) {
        send_key_down_event(self, event, true, &self.keymap_keys, self.xkb_current_layout);
    }

    fn update_keymap(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _keyboard: &WlKeyboard, keymap: Keymap<'_>) {
        let keymap_str = keymap.as_string();
        let keymap_len = keymap_str.len();
        debug!("KeyboardHandler::update_keymap: keymap_len={keymap_len}");
        let context = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
        let xkb_keymap =
            xkb::Keymap::new_from_string(&context, keymap_str, xkb::KEYMAP_FORMAT_TEXT_V1, xkb::KEYMAP_COMPILE_NO_FLAGS).unwrap();

        self.keymap_keys.clear();
        // TODO: add layout changed event
        self.xkb_current_layout = 0;

        // let mut level_modifier_mapping = HashMap::new();
        // level_modifier_mapping.insert(xkb_keymap.mod_get_index(xkb::MOD_NAME_SHIFT), KeyModifier::Shift);
        // level_modifier_mapping.insert(xkb_keymap.mod_get_index(xkb::MOD_NAME_CAPS), KeyModifier::CapsLock);
        // level_modifier_mapping.insert(xkb_keymap.mod_get_index(xkb::MOD_NAME_CTRL), KeyModifier::Ctrl);
        // level_modifier_mapping.insert(xkb_keymap.mod_get_index(xkb::MOD_NAME_ALT), KeyModifier::Alt);
        // level_modifier_mapping.insert(xkb_keymap.mod_get_index(xkb::MOD_NAME_NUM), KeyModifier::NumLock);
        // // level_modifier_mapping.insert(xkb_keymap.mod_get_index(xkb::MOD_NAME_MOD3), KeyModifier::);
        // level_modifier_mapping.insert(xkb_keymap.mod_get_index(xkb::MOD_NAME_LOGO), KeyModifier::Logo);
        // // level_modifier_mapping.insert(xkb_keymap.mod_get_index(xkb::MOD_NAME_ISO_LEVEL3_SHIFT), KeyModifier::Shift);

        for layout in 0..xkb_keymap.num_layouts() {
            // let layout_name = xkb_keymap.layout_get_name(layout);
            // debug!("KeyboardHandler::update_keymap: layout {layout} = {layout_name}");
            xkb_keymap.key_for_each(|keymap, keycode| {
                for level in 0..keymap.num_levels_for_key(keycode, layout) {
                    for keysym in keymap.key_get_syms_by_level(keycode, layout, level) {
                        let char_u32 = xkb::keysym_to_utf32(*keysym);
                        if char_u32 > 0 {
                            let keymap_key = KeymapKey {
                                keycode: KeyCode(keycode.raw() - 8),
                                keysym: *keysym,
                                char_u32,
                                layout,
                                level,
                            };
                            // debug!("keymap_key={keymap_key:?}");
                            self.keymap_keys.push(keymap_key);
                        }
                    }
                }
            });
        }
    }
}

delegate_keyboard!(ApplicationState);
