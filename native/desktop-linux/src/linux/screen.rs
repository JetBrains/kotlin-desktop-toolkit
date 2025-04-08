use desktop_common::{
    ffi_utils::{AutoDropArray, AutoDropStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use smithay_client_toolkit::output::OutputInfo;

use super::{
    application::Application,
    application_api::AppPtr,
    geometry::{LogicalPixels, LogicalPoint, LogicalSize},
};

pub type ScreenId = u32;

#[repr(C)]
pub struct ScreenInfo {
    pub screen_id: ScreenId,
    pub is_primary: bool,
    pub name: AutoDropStrPtr,
    // relative to primary screen
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub scale: f64,
    // todo color space?
    // todo stable uuid?
}

impl ScreenInfo {
    #[must_use]
    pub fn new(info: OutputInfo) -> Self {
        Self {
            screen_id: info.id,
            // The screen containing the menu bar is always the first object (index 0) in the array returned by the screens method.
            is_primary: false,
            name: info.name.map_or_else(
                || RustAllocatedStrPtr::null().to_auto_drop(),
                |s| RustAllocatedStrPtr::allocate(s.as_bytes()).unwrap().to_auto_drop(),
            ),
            origin: info.logical_position.map_or(
                LogicalPoint {
                    x: LogicalPixels(0.0),
                    y: LogicalPixels(0.0),
                },
                |pos| LogicalPoint {
                    x: LogicalPixels(pos.0.into()),
                    y: LogicalPixels(pos.1.into()),
                },
            ),
            size: info
                .logical_size
                .map(|size| LogicalSize {
                    width: LogicalPixels(f64::from(size.0)),
                    height: LogicalPixels(f64::from(size.1)),
                })
                .unwrap_or_default(),
            scale: info.scale_factor.into(),
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
