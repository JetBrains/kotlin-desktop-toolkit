use std::{ffi::CString, str::FromStr};

use desktop_common::{
    ffi_utils::{AutoDropArray, AutoDropStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};

use super::{
    application::Application,
    application_api::AppPtr,
    events::{LogicalPixels, LogicalPoint, LogicalSize},
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
            .enumerate()
            .map(|(num, info)| {
                let name = info.name.unwrap_or_default();
                ScreenInfo {
                    screen_id: info.id,
                    // The screen containing the menu bar is always the first object (index 0) in the array returned by the screens method.
                    is_primary: num == 0,
                    name: RustAllocatedStrPtr::allocate(CString::from_str(&name).unwrap().as_bytes())
                        .unwrap()
                        .to_auto_drop(),
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
                    size: info.logical_size.map_or(
                        LogicalSize {
                            width: LogicalPixels(0.0),
                            height: LogicalPixels(0.0),
                        },
                        |size| LogicalSize {
                            width: LogicalPixels(size.0.into()),
                            height: LogicalPixels(size.1.into()),
                        },
                    ),
                    scale: info.scale_factor.into(),
                }
            })
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
