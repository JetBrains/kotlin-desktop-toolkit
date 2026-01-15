use anyhow::Context;
use desktop_common::{ffi_utils::BorrowedOpaquePtr, logger::ffi_boundary};
use windows::Win32::UI::WindowsAndMessaging::MSG;

#[unsafe(no_mangle)]
pub extern "C" fn keydown_translate_message(msg: BorrowedOpaquePtr) -> bool {
    ffi_boundary("keydown_translate_message", || {
        unsafe { msg.borrow::<MSG>() }.context("MSG is null").map(|msg| {
            // https://learn.microsoft.com/en-us/windows/win32/winmsg/translatemessageex
            // If bit 0 is set, a menu is active. In this mode Alt+Numeric keypad key combinations are not handled.
            // If bit 1 is set, TranslateMessageEx will return FALSE when it does not post WM_CHAR or WM_SYSCHAR to the message loop.
            // If bit 2 is set, keyboard state is not changed (Windows 10, version 1607 and newer)
            let flags = 1 << 1;
            unsafe { TranslateMessageEx(msg, flags) }.as_bool()
        })
    })
}

#[allow(non_snake_case)]
#[inline]
unsafe fn TranslateMessageEx(lpmsg: *const MSG, flags: u32) -> windows::core::BOOL {
    windows::core::link!("user32.dll" "system" fn TranslateMessageEx(lpmsg : *const MSG, flags : u32) -> windows::core::BOOL);
    unsafe { TranslateMessageEx(lpmsg, flags) }
}
