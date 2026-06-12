use crate::gtk::application::send_event;
use crate::gtk::events::{EventHandler, KeyCode, KeyDownEvent, KeyModifiers, KeyUpEvent, WindowId};
use gtk4::glib::translate::IntoGlib;
use gtk4::{gdk as gdk4, glib};
use log::{debug, warn};

#[must_use]
pub fn key_modifiers_from_gdk(modifiers: gdk4::ModifierType) -> KeyModifiers {
    let mut key_modifiers = KeyModifiers::empty();
    if modifiers.contains(gdk4::ModifierType::CONTROL_MASK) {
        key_modifiers |= KeyModifiers::Ctrl;
    }
    if modifiers.contains(gdk4::ModifierType::ALT_MASK) {
        key_modifiers |= KeyModifiers::Alt;
    }
    if modifiers.contains(gdk4::ModifierType::SHIFT_MASK) {
        key_modifiers |= KeyModifiers::Shift;
    }
    if modifiers.contains(gdk4::ModifierType::LOCK_MASK) {
        key_modifiers |= KeyModifiers::CapsLock;
    }
    if modifiers.contains(gdk4::ModifierType::SUPER_MASK) {
        key_modifiers |= KeyModifiers::Logo;
    }

    key_modifiers
}

fn to_xkb_keycode(raw: u32) -> KeyCode {
    if cfg!(all(unix, not(target_os = "macos"), not(target_os = "ios"))) {
        return KeyCode(raw);
    }

    let Ok(raw_u16) = u16::try_from(raw) else {
        warn!("decode_key_code: raw value too large ({raw})");
        return KeyCode(raw);
    };

    let key_mapping = if cfg!(target_os = "macos") {
        keycode::KeyMapping::Mac(raw_u16)
    } else if cfg!(target_os = "windows") {
        keycode::KeyMapping::Win(raw_u16)
    } else {
        warn!("decode_key_code: unknown OS");
        return KeyCode(raw);
    };

    if let Ok(keymap) = keycode::KeyMap::from_key_mapping(key_mapping) {
        KeyCode(keymap.xkb.into())
    } else {
        warn!("decode_key_code error for {raw}");
        KeyCode(raw)
    }
}

pub fn set_keyboard_event_handlers(window_id: WindowId, event_handler: EventHandler) -> gtk4::EventControllerKey {
    let event_controller_key = gtk4::EventControllerKey::new();

    event_controller_key.connect_key_pressed(move |_event_controller_key, keyval, keycode, _modifiers| {
        debug!("key_pressed: {keycode}");

        let code = to_xkb_keycode(keycode);
        let key = keyval.into_glib();
        let char: Option<char> = keyval.to_unicode();
        let event = KeyDownEvent {
            window_id,
            code,
            has_character: char.is_some(),
            character: char.unwrap_or_default(),
            key,
        };
        send_event(event_handler, event);
        glib::Propagation::Proceed
    });

    event_controller_key.connect_key_released(move |_event_controller_key, keyval, keycode, _modifiers| {
        debug!("key_released: {keycode}");

        let code = to_xkb_keycode(keycode);
        let key = keyval.into_glib();
        let event = KeyUpEvent { window_id, code, key };
        send_event(event_handler, event);
    });

    event_controller_key
}
