use desktop_common::{
    ffi_utils::{AutoDropArray, RustAllocatedStrPtr},
    logger::{PanicDefault, ffi_boundary},
};

use super::{
    geometry::{LogicalPoint, LogicalSize},
    screen::{ScreenInfo, enumerate_screens},
};

type ScreenInfoArray = AutoDropArray<ScreenInfo>;

impl PanicDefault for ScreenInfo {
    fn default() -> Self {
        Self {
            is_primary: Default::default(),
            name: RustAllocatedStrPtr::null().to_auto_drop(),
            origin: LogicalPoint::default(),
            size: LogicalSize::default(),
            scale: Default::default(),
            maximum_frames_per_second: Default::default(),
        }
    }
}

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

#[unsafe(no_mangle)]
pub extern "C" fn screen_info_drop(screen_info: ScreenInfo) {
    ffi_boundary("screen_info_drop", || {
        core::mem::drop(screen_info);
        Ok(())
    });
}
