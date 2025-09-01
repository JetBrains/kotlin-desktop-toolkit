use windows::Win32::{
    Foundation::LPARAM,
    UI::WindowsAndMessaging::{KF_ALTDOWN, KF_EXTENDED, KF_REPEAT, KF_UP},
};

use super::utils::{HIWORD, LOBYTE, LOWORD};

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct VirtualKey(pub u16);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalKeyStatus {
    scan_code: u32,
    is_extended_key: bool,
    is_menu_key_down: bool,
    was_key_down: bool,
    is_key_released: bool,
    repeat_count: u32,
}

impl PhysicalKeyStatus {
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    #[must_use]
    pub const fn parse(lparam: LPARAM) -> Self {
        let key_flags = HIWORD!(lparam.0);
        let repeat_count = LOWORD!(lparam.0) as u32;

        let scan_code = LOBYTE!(key_flags) as u32;
        let is_extended_key = (key_flags as u32 & KF_EXTENDED) == KF_EXTENDED;
        let is_menu_key_down = (key_flags as u32 & KF_ALTDOWN) == KF_ALTDOWN;
        let was_key_down = (key_flags as u32 & KF_REPEAT) == KF_REPEAT;
        let is_key_released = (key_flags as u32 & KF_UP) == KF_UP;

        Self {
            scan_code,
            is_extended_key,
            is_menu_key_down,
            was_key_down,
            is_key_released,
            repeat_count,
        }
    }
}
