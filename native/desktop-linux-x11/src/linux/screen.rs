use crate::linux::events::ScreenId;
use crate::linux::{
    application::Application,
    application_api::AppPtr,
    geometry::{LogicalPoint, LogicalSize},
};
use desktop_common::{
    ffi_utils::{AutoDropArray, AutoDropStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use winit_core::monitor::MonitorHandle;

#[repr(C)]
pub struct ScreenInfo {
    pub screen_id: ScreenId,
    pub name: AutoDropStrPtr,
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub scale: f64,
    pub millihertz: u32,
    // todo color space?
    // todo stable uuid?
}

impl ScreenInfo {
    #[must_use]
    fn new(monitor: MonitorHandle) -> Self {
        let current_mode = monitor.current_video_mode();
        let origin = if let Some(position) = monitor.position() {
            position.to_logical(monitor.scale_factor()).into()
        } else {
            LogicalPoint::default()
        };
        Self {
            screen_id: ScreenId(monitor.native_id()),
            name: monitor.name().map_or_else(
                || RustAllocatedStrPtr::null().to_auto_drop(),
                |s| RustAllocatedStrPtr::allocate(s.as_bytes()).unwrap().to_auto_drop(),
            ),
            origin,
            size: current_mode
                .map(|mode| mode.size().to_logical(monitor.scale_factor()).into())
                .unwrap_or_default(),
            scale: monitor.scale_factor(),
            millihertz: current_mode
                .and_then(|mode| mode.refresh_rate_millihertz().map(std::num::NonZero::get))
                .unwrap_or_default(),
        }
    }
}

type ScreenInfoArray = AutoDropArray<ScreenInfo>;

#[unsafe(no_mangle)]
pub extern "C" fn screen_list(app_ptr: AppPtr) -> ScreenInfoArray {
    ffi_boundary("screen_list", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let screen_infos: Box<_> = if let Some(w) = app.state.windows.values().next() {
            w.window.available_monitors().map(ScreenInfo::new).collect()
        } else {
            Box::new([])
        };
        Ok(ScreenInfoArray::new(screen_infos))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    ffi_boundary("screen_list_drop", || {
        drop(arr);
        Ok(())
    });
}
