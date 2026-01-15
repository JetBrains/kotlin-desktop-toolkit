use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::WindowsAndMessaging::{KF_ALTDOWN, KF_EXTENDED, KF_REPEAT, KF_UP},
};

use super::utils::{HIWORD, LOBYTE, LOWORD};

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct VirtualKey(pub u16);

impl From<WPARAM> for VirtualKey {
    fn from(wparam: WPARAM) -> Self {
        Self(LOWORD!(wparam.0))
    }
}

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
    #[must_use]
    pub const fn scan_code(&self) -> u32 {
        self.scan_code
    }
}

impl From<LPARAM> for PhysicalKeyStatus {
    fn from(lparam: LPARAM) -> Self {
        let key_flags = u32::from(HIWORD!(lparam.0));
        let repeat_count = u32::from(LOWORD!(lparam.0));

        let scan_code = u32::from(LOBYTE!(key_flags));
        let is_extended_key = (key_flags & KF_EXTENDED) == KF_EXTENDED;
        let is_menu_key_down = (key_flags & KF_ALTDOWN) == KF_ALTDOWN;
        let was_key_down = (key_flags & KF_REPEAT) == KF_REPEAT;
        let is_key_released = (key_flags & KF_UP) == KF_UP;

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
