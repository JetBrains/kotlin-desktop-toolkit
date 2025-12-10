use windows::Win32::UI::WindowsAndMessaging::ShowCursor;

use desktop_common::logger::{PanicDefault, ffi_boundary};

#[repr(transparent)]
pub struct CursorDisplayCounter(i32);

impl PanicDefault for CursorDisplayCounter {
    fn default() -> Self {
        Self(0)
    }
}

impl From<i32> for CursorDisplayCounter {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cursor_show() -> CursorDisplayCounter {
    ffi_boundary("cursor_show", || {
        let display_counter = unsafe { ShowCursor(true) };
        Ok(display_counter.into())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn cursor_hide() -> CursorDisplayCounter {
    ffi_boundary("cursor_hide", || {
        let display_counter = unsafe { ShowCursor(false) };
        Ok(display_counter.into())
    })
}
