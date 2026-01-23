use anyhow::Context;
use windows::Win32::UI::{
    Input::KeyboardAndMouse::{GetKeyboardLayout, GetKeyboardState, ToUnicodeEx},
    WindowsAndMessaging::MSG,
};

use desktop_common::{ffi_utils::RustAllocatedStrPtr, logger::ffi_boundary};

use super::{
    event_loop::EventLoop,
    keyboard::{PhysicalKeyStatus, VirtualKey},
};

#[unsafe(no_mangle)]
pub extern "C" fn keyevent_translate_message(msg_id: u64) -> bool {
    ffi_boundary("keyevent_translate_message", || {
        EventLoop::with_keyevent_message(msg_id, |msg| {
            // https://learn.microsoft.com/en-us/windows/win32/winmsg/translatemessageex
            // If bit 0 is set, a menu is active. In this mode Alt+Numeric keypad key combinations are not handled.
            // If bit 1 is set, TranslateMessageEx will return FALSE when it does not post WM_CHAR or WM_SYSCHAR to the message loop.
            // If bit 2 is set, keyboard state is not changed (Windows 10, version 1607 and newer)
            let flags = 1 << 1;
            let result = unsafe { TranslateMessageEx(msg, flags) };
            Ok(result.as_bool())
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn keydown_to_unicode(msg_id: u64) -> RustAllocatedStrPtr {
    ffi_boundary("keydown_to_unicode", || {
        EventLoop::with_keyevent_message(msg_id, |msg| {
            // https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-tounicodeex
            // If bit 0 is set, a menu is active. In this mode Alt+Numeric keypad key combinations are not handled.
            // If bit 1 is set, ToUnicodeEx will translate scancodes marked as key break events in addition to its usual treatment of key make events.
            // If bit 2 is set, keyboard state is not changed (Windows 10, version 1607 and newer)
            let flags = 1 << 2;

            let virtual_key = VirtualKey::from(msg.wParam);
            let key_status = PhysicalKeyStatus::from(msg.lParam);

            let mut key_state = [0u8; 256];
            let mut unicode_buf = [0u16; 64];

            let char_count = unsafe {
                GetKeyboardState(&mut key_state)?;
                let keyboard_layout = GetKeyboardLayout(0);
                ToUnicodeEx(
                    u32::from(virtual_key.0),
                    key_status.scan_code(),
                    &key_state,
                    &mut unicode_buf,
                    flags,
                    Some(keyboard_layout),
                )
            };

            #[allow(clippy::cast_sign_loss)]
            let unicode_str = String::from_utf16(&unicode_buf[..char_count.unsigned_abs() as usize])?;
            RustAllocatedStrPtr::allocate(unicode_str).context("Failed to allocate a Rust string")
        })
    })
}

#[allow(non_snake_case)]
#[inline]
unsafe fn TranslateMessageEx(lpmsg: *const MSG, flags: u32) -> windows::core::BOOL {
    windows::core::link!("user32.dll" "system" fn TranslateMessageEx(lpmsg : *const MSG, flags : u32) -> windows::core::BOOL);
    unsafe { TranslateMessageEx(lpmsg, flags) }
}
