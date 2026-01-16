use crate::linux::events::{KeyModifier, KeyModifierBitflag};
use enumflags2::BitFlags;
use gtk4::gdk as gdk4;

#[must_use]
pub fn key_modifiers_from_gdk(modifiers: gdk4::ModifierType) -> BitFlags<KeyModifier> {
    let mut key_modifiers = BitFlags::<KeyModifier>::EMPTY;
    key_modifiers.set(KeyModifier::Ctrl, modifiers.contains(gdk4::ModifierType::CONTROL_MASK));
    key_modifiers.set(KeyModifier::Alt, modifiers.contains(gdk4::ModifierType::ALT_MASK));
    key_modifiers.set(KeyModifier::Shift, modifiers.contains(gdk4::ModifierType::SHIFT_MASK));
    key_modifiers.set(KeyModifier::CapsLock, modifiers.contains(gdk4::ModifierType::LOCK_MASK));
    key_modifiers.set(KeyModifier::Logo, modifiers.contains(gdk4::ModifierType::SUPER_MASK));
    // key_modifiers.set(KeyModifier::NumLock, modifiers.num_lock);

    key_modifiers
}

#[must_use]
pub const fn key_modifier_from_gdk(key: gdk4::Key) -> Option<KeyModifier> {
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
