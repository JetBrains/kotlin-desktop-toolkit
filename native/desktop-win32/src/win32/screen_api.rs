use desktop_common::{ffi_utils::AutoDropArray, logger::ffi_boundary};

use super::screen::{ScreenInfo, enumerate_screens};

type ScreenInfoArray = AutoDropArray<ScreenInfo>;

#[unsafe(no_mangle)]
pub extern "C" fn screen_list() -> ScreenInfoArray {
    ffi_boundary("screen_list", || {
        let screen_infos = enumerate_screens()?;
        Ok(ScreenInfoArray::new(screen_infos))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    ffi_boundary("screen_list_drop", || {
        core::mem::drop(arr);
        Ok(())
    });
}
