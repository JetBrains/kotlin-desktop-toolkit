use desktop_common::{
    ffi_utils::AutoDropArray,
    logger::{PanicDefault, ffi_boundary},
};

use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, GetKeyboardState};

#[repr(C)]
pub struct KeyState {
    pub is_down: bool,
    pub is_toggled: bool,
}

impl PanicDefault for KeyState {
    fn default() -> Self {
        Self {
            is_down: false,
            is_toggled: false,
        }
    }
}

/// cbindgen:ignore
const DOWN_BIT: u16 = 0b1000_0000_0000_0000; // high-order bit
/// cbindgen:ignore
const TOGGLED_BIT: u16 = 0b0000_0000_0000_0001; // low-order bit

#[unsafe(no_mangle)]
pub extern "C" fn keyboard_get_key_state(vkey: i32) -> KeyState {
    ffi_boundary("keyboard_get_key_state", || {
        let native_state = unsafe { GetKeyState(vkey) }.cast_unsigned();
        let is_down = (native_state & DOWN_BIT) != 0;
        let is_toggled = (native_state & TOGGLED_BIT) != 0;
        let key_state = KeyState { is_down, is_toggled };
        Ok(key_state)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keyboard_get_state() -> AutoDropArray<u8> {
    ffi_boundary("keyboard_get_state", || {
        let mut keyboard_state = Box::new([0u8; 256]);
        unsafe { GetKeyboardState(keyboard_state.as_mut())? };
        Ok(AutoDropArray::new(keyboard_state))
    })
}
