use crate::linux::events::{KeyModifier, KeyModifierBitflag};
use enumflags2::BitFlags;
use winit_core::keyboard::NativeKey;

fn either_is_pressed(a: winit_core::keyboard::ModifiersKeyState, b: winit_core::keyboard::ModifiersKeyState) -> bool {
    a == winit_core::keyboard::ModifiersKeyState::Pressed || b == winit_core::keyboard::ModifiersKeyState::Pressed
}

impl KeyModifierBitflag {
    #[must_use]
    pub fn from_winit(modifiers: winit_core::event::Modifiers) -> Self {
        let mut key_modifiers = BitFlags::<KeyModifier>::EMPTY;
        key_modifiers.set(
            KeyModifier::Ctrl,
            either_is_pressed(modifiers.lcontrol_state(), modifiers.rcontrol_state()),
        );
        key_modifiers.set(KeyModifier::Alt, either_is_pressed(modifiers.lalt_state(), modifiers.ralt_state()));
        key_modifiers.set(
            KeyModifier::Shift,
            either_is_pressed(modifiers.lshift_state(), modifiers.rshift_state()),
        );
        // key_modifiers.set(KeyModifier::CapsLock, modifiers.);
        key_modifiers.set(
            KeyModifier::Logo,
            either_is_pressed(modifiers.lsuper_state(), modifiers.rsuper_state()),
        );
        // key_modifiers.set(KeyModifier::NumLock, modifiers.num_lock);

        Self(key_modifiers.bits_c())
    }
}

#[allow(clippy::too_many_lines)]
pub fn winit_key_to_keysym(key: winit_core::keyboard::Key, location: winit_core::keyboard::KeyLocation) -> u32 {
    use winit_core::keyboard::{KeyLocation, NamedKey};
    match key {
        winit_core::keyboard::Key::Named(named_key) => match named_key {
            NamedKey::Alt => match location {
                KeyLocation::Left => xkeysym::key::Alt_L,
                KeyLocation::Right => xkeysym::key::Alt_R,
                _ => xkeysym::key::NoSymbol,
            },
            NamedKey::AltGraph => xkeysym::key::ISO_Level3_Shift,
            NamedKey::CapsLock => xkeysym::key::Caps_Lock,
            NamedKey::Control => match location {
                KeyLocation::Left => xkeysym::key::Control_L,
                KeyLocation::Right => xkeysym::key::Control_R,
                _ => xkeysym::key::NoSymbol,
            },
            NamedKey::NumLock => xkeysym::key::Num_Lock,
            NamedKey::ScrollLock => xkeysym::key::Scroll_Lock,
            NamedKey::Shift => match location {
                KeyLocation::Left => xkeysym::key::Shift_L,
                KeyLocation::Right => xkeysym::key::Shift_R,
                _ => xkeysym::key::NoSymbol,
            },
            NamedKey::Meta => match location {
                KeyLocation::Left => xkeysym::key::Meta_L,
                KeyLocation::Right => xkeysym::key::Meta_R,
                _ => xkeysym::key::NoSymbol,
            },
            NamedKey::Enter => match location {
                KeyLocation::Standard => xkeysym::key::Return,
                KeyLocation::Numpad => xkeysym::key::KP_Enter,
                _ => xkeysym::key::NoSymbol,
            },
            NamedKey::Tab => xkeysym::key::Tab,
            NamedKey::ArrowDown => xkeysym::key::Down,
            NamedKey::ArrowLeft => xkeysym::key::Left,
            NamedKey::ArrowRight => xkeysym::key::Right,
            NamedKey::ArrowUp => xkeysym::key::Up,
            NamedKey::End => xkeysym::key::End,
            NamedKey::Home => xkeysym::key::Home,
            NamedKey::PageDown => xkeysym::key::Page_Down,
            NamedKey::PageUp => xkeysym::key::Page_Up,
            NamedKey::Backspace => xkeysym::key::BackSpace,
            NamedKey::Clear => xkeysym::key::Clear,
            NamedKey::Copy => xkeysym::key::XF86_Copy,
            NamedKey::Cut => xkeysym::key::XF86_Cut,
            NamedKey::Delete => xkeysym::key::Delete,
            NamedKey::Insert => xkeysym::key::Insert,
            NamedKey::Paste => xkeysym::key::XF86_Paste,
            NamedKey::Redo => xkeysym::key::Redo,
            NamedKey::Undo => xkeysym::key::Undo,
            NamedKey::Cancel => xkeysym::key::Cancel,
            NamedKey::ContextMenu => xkeysym::key::XF86_ContextMenu,
            NamedKey::Escape => xkeysym::key::Escape,
            NamedKey::Execute => xkeysym::key::Execute,
            NamedKey::Find => xkeysym::key::Find,
            NamedKey::Help => xkeysym::key::Help,
            NamedKey::Select => xkeysym::key::Select,
            NamedKey::ZoomIn => xkeysym::key::XF86_ZoomIn,
            NamedKey::ZoomOut => xkeysym::key::XF86_ZoomOut,
            NamedKey::Close => xkeysym::key::XF86_Close,
            NamedKey::New => xkeysym::key::XF86_New,
            NamedKey::Open => xkeysym::key::XF86_Open,
            NamedKey::Save => xkeysym::key::XF86_Save,
            NamedKey::SpellCheck => xkeysym::key::XF86_Spell,
            NamedKey::BrowserBack => xkeysym::key::XF86_Back,
            NamedKey::BrowserFavorites => xkeysym::key::XF86_Favorites,
            NamedKey::BrowserForward => xkeysym::key::XF86_Forward,
            NamedKey::BrowserHome => xkeysym::key::XF86_HomePage,
            NamedKey::BrowserRefresh => xkeysym::key::XF86_Refresh,
            NamedKey::BrowserSearch => xkeysym::key::XF86_Search,
            NamedKey::BrowserStop => xkeysym::key::XF86_Stop,
            NamedKey::F1 => xkeysym::key::F1,
            NamedKey::F2 => xkeysym::key::F2,
            NamedKey::F3 => xkeysym::key::F3,
            NamedKey::F4 => xkeysym::key::F4,
            NamedKey::F5 => xkeysym::key::F5,
            NamedKey::F6 => xkeysym::key::F6,
            NamedKey::F7 => xkeysym::key::F7,
            NamedKey::F8 => xkeysym::key::F8,
            NamedKey::F9 => xkeysym::key::F9,
            NamedKey::F10 => xkeysym::key::F10,
            NamedKey::F11 => xkeysym::key::F11,
            NamedKey::F12 => xkeysym::key::F12,
            NamedKey::F13 => xkeysym::key::F13,
            NamedKey::F14 => xkeysym::key::F14,
            NamedKey::F15 => xkeysym::key::F15,
            NamedKey::F16 => xkeysym::key::F16,
            NamedKey::F17 => xkeysym::key::F17,
            NamedKey::F18 => xkeysym::key::F18,
            NamedKey::F19 => xkeysym::key::F19,
            NamedKey::F20 => xkeysym::key::F20,
            NamedKey::F21 => xkeysym::key::F21,
            NamedKey::F22 => xkeysym::key::F22,
            NamedKey::F23 => xkeysym::key::F23,
            NamedKey::F24 => xkeysym::key::F24,
            NamedKey::F25 => xkeysym::key::F25,
            NamedKey::F26 => xkeysym::key::F26,
            NamedKey::F27 => xkeysym::key::F27,
            NamedKey::F28 => xkeysym::key::F28,
            NamedKey::F29 => xkeysym::key::F29,
            NamedKey::F30 => xkeysym::key::F30,
            NamedKey::F31 => xkeysym::key::F31,
            NamedKey::F32 => xkeysym::key::F32,
            NamedKey::F33 => xkeysym::key::F33,
            NamedKey::F34 => xkeysym::key::F34,
            NamedKey::F35 => xkeysym::key::F35,
            // NamedKey::AllCandidates => {}
            // NamedKey::Alphanumeric => {}
            // NamedKey::CodeInput => {}
            // NamedKey::Compose => {}
            // NamedKey::Convert => {}
            // NamedKey::GroupFirst => {}
            // NamedKey::GroupLast => {}
            // NamedKey::GroupNext => {}
            // NamedKey::GroupPrevious => {}
            // NamedKey::ModeChange => xkeysym::key::Mode_switch,
            // NamedKey::NonConvert => {}
            // NamedKey::PreviousCandidate => {}
            // NamedKey::SingleCandidate => {}
            // NamedKey::Hiragana => {}
            // NamedKey::HiraganaKatakana => {}
            // NamedKey::KanaMode => {}
            // NamedKey::KanjiMode => {}
            // NamedKey::Romaji => {}
            // NamedKey::Zenkaku => {}
            // NamedKey::ZenkakuHankaku => {}
            _ => xkeysym::key::NoSymbol,
        },
        winit_core::keyboard::Key::Unidentified(native_key) => {
            if let NativeKey::Xkb(raw) = native_key {
                raw
            } else {
                xkeysym::key::NoSymbol
            }
        }
        winit_core::keyboard::Key::Character(_) => xkeysym::key::NoSymbol,
        winit_core::keyboard::Key::Dead(_) => xkeysym::key::NoSymbol,
    }
}
