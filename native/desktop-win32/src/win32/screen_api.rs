use desktop_common::{
    ffi_utils::{AutoDropArray, RustAllocatedStrPtr},
    logger::{PanicDefault, ffi_boundary},
};

use super::{
    geometry::{LogicalPoint, LogicalSize, PhysicalPoint},
    screen::{self, ScreenInfo},
    window_api::{WindowPtr, with_window},
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
        let screen_infos = screen::enumerate_screens()?;
        Ok(ScreenInfoArray::new(screen_infos))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_map_to_client(window_ptr: WindowPtr, point: PhysicalPoint) -> PhysicalPoint {
    with_window(&window_ptr, "screen_map_to_client", |window| {
        screen::screen_to_client(window, point)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    ffi_boundary("screen_list_drop", || {
        drop(arr);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_info_drop(screen_info: ScreenInfo) {
    ffi_boundary("screen_info_drop", || {
        drop(screen_info);
        Ok(())
    });
}
