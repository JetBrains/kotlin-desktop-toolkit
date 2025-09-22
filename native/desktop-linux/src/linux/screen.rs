use desktop_common::{
    ffi_utils::{AutoDropArray, AutoDropStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use smithay_client_toolkit::output::{Mode, OutputInfo};

use crate::linux::{
    application::Application,
    application_api::AppPtr,
    geometry::{LogicalPixels, LogicalPoint, LogicalSize},
};

pub type ScreenId = u32;

#[repr(C)]
pub struct ScreenInfo {
    pub screen_id: ScreenId,
    pub name: AutoDropStrPtr,
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub scale: f64,
    pub maximum_frames_per_second: i32,
    // todo color space?
    // todo stable uuid?
}

impl ScreenInfo {
    #[allow(clippy::cast_possible_truncation)]
    fn get_refresh_rate_fps(mode: &Mode) -> i32 {
        (f64::from(mode.refresh_rate) / 1000.).round() as i32
    }

    #[must_use]
    pub fn new(info: OutputInfo) -> Self {
        let current_mode = info.modes.iter().find(|m| m.current);
        Self {
            screen_id: info.id,
            name: info.name.map_or_else(
                || RustAllocatedStrPtr::null().to_auto_drop(),
                |s| RustAllocatedStrPtr::allocate(s.as_bytes()).unwrap().to_auto_drop(),
            ),
            origin: info.logical_position.map(Into::into).unwrap_or_default(),
            size: info
                .logical_size
                .map(|size| LogicalSize {
                    width: LogicalPixels(size.0.into()),
                    height: LogicalPixels(size.1.into()),
                })
                .unwrap_or_default(),
            scale: info.scale_factor.into(),
            maximum_frames_per_second: current_mode.map(Self::get_refresh_rate_fps).unwrap_or_default(),
        }
    }
}

type ScreenInfoArray = AutoDropArray<ScreenInfo>;

#[unsafe(no_mangle)]
pub extern "C" fn screen_list(app_ptr: AppPtr) -> ScreenInfoArray {
    ffi_boundary("screen_list", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let screen_infos: Box<_> = app
            .state
            .output_state
            .outputs()
            .filter_map(|output| app.state.output_state.info(&output))
            .map(ScreenInfo::new)
            .collect();
        Ok(ScreenInfoArray::new(screen_infos))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    ffi_boundary("screen_list_drop", || {
        std::mem::drop(arr);
        Ok(())
    });
}
