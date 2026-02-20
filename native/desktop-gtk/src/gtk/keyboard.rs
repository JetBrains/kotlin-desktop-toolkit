use crate::gtk::application::send_event;
use crate::gtk::events::{
    EventHandler, KeyCode, KeyDownEvent, KeyModifier, KeyModifierBitflag, KeyUpEvent, ModifiersChangedEvent, WindowId,
};
use enumflags2::BitFlags;
use gtk4::glib::translate::IntoGlib;
use gtk4::{gdk as gdk4, glib};
use log::{debug, warn};

#[must_use]
fn key_modifiers_from_gdk(modifiers: gdk4::ModifierType) -> BitFlags<KeyModifier> {
    let mut key_modifiers = BitFlags::<KeyModifier>::EMPTY;
    key_modifiers.set(KeyModifier::Ctrl, modifiers.contains(gdk4::ModifierType::CONTROL_MASK));
    key_modifiers.set(KeyModifier::Alt, modifiers.contains(gdk4::ModifierType::ALT_MASK));
    key_modifiers.set(KeyModifier::Shift, modifiers.contains(gdk4::ModifierType::SHIFT_MASK));
    key_modifiers.set(KeyModifier::CapsLock, modifiers.contains(gdk4::ModifierType::LOCK_MASK));
    key_modifiers.set(KeyModifier::Logo, modifiers.contains(gdk4::ModifierType::SUPER_MASK));

    key_modifiers
}

#[must_use]
const fn key_modifier_from_gdk(key: gdk4::Key) -> Option<KeyModifier> {
    match key {
        gdk4::Key::Control_L | gdk4::Key::Control_R => Some(KeyModifier::Ctrl),
        gdk4::Key::Alt_L | gdk4::Key::Alt_R => Some(KeyModifier::Alt),
        gdk4::Key::Shift_L | gdk4::Key::Shift_R => Some(KeyModifier::Shift),
        gdk4::Key::Caps_Lock => Some(KeyModifier::CapsLock),
        gdk4::Key::Super_L | gdk4::Key::Super_R => Some(KeyModifier::Logo),
        _ => None,
    }
}

impl KeyModifierBitflag {
    #[must_use]
    pub const fn new(key_modifiers: BitFlags<KeyModifier>) -> Self {
        Self(key_modifiers.bits_c())
    }
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

    event_controller_key.connect_key_pressed(move |_event_controller_key, keyval, keycode, modifiers| {
        debug!("connect_key_pressed: {keycode}, modifiers={modifiers:?}");

        let mut key_modifiers = key_modifiers_from_gdk(modifiers);
        // Cannot use the "modifiers" signal, see https://gitlab.gnome.org/GNOME/gtk/-/issues/5139
        if let Some(modifier) = key_modifier_from_gdk(keyval) {
            key_modifiers.set(modifier, true);
            let event = ModifiersChangedEvent {
                window_id,
                modifiers: KeyModifierBitflag::new(key_modifiers),
            };
            send_event(event_handler, event);
        }

        let code = to_xkb_keycode(keycode);
        let key = keyval.into_glib();
        let char: Option<char> = keyval.to_unicode();
        let event = KeyDownEvent {
            window_id,
            code,
            has_character: char.is_some(),
            character: char.unwrap_or_default(),
            key,
            // key_without_modifiers,
            modifiers: KeyModifierBitflag::new(key_modifiers),
            is_repeat: false, //event.repeat, // TODO
        };
        send_event(event_handler, event);
        glib::Propagation::Proceed
    });

    event_controller_key.connect_key_released(move |_event_controller_key, keyval, keycode, modifiers| {
        debug!("connect_key_released: {keycode}");

        // Cannot use the "modifiers" signal, see https://gitlab.gnome.org/GNOME/gtk/-/issues/5139
        if let Some(modifier) = key_modifier_from_gdk(keyval) {
            let mut key_modifiers = key_modifiers_from_gdk(modifiers);
            key_modifiers.set(modifier, false);
            let event = ModifiersChangedEvent {
                window_id,
                modifiers: KeyModifierBitflag::new(key_modifiers),
            };
            send_event(event_handler, event);
        }

        let code = to_xkb_keycode(keycode);
        let key = keyval.into_glib();
        let event = KeyUpEvent {
            window_id,
            code,
            key,
            // key_without_modifiers,
        };
        send_event(event_handler, event);
    });

    event_controller_key
}
