use crate::linux::application::Application;
use crate::linux::application_api::AppPtr;
use crate::linux::events::ScreenId;
use crate::linux::geometry::{LogicalPixels, LogicalPoint, LogicalSize};
use anyhow::Context;
use desktop_common::{
    ffi_utils::{AutoDropArray, AutoDropStrPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use gtk4::gdk as gdk4;
use gtk4::prelude::{Cast, DisplayExt, MonitorExt};
use std::hash::{DefaultHasher, Hash, Hasher};

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
    fn new(monitor: &gtk4::gdk::Monitor) -> Self {
        let geometry = monitor.geometry();
        let origin = LogicalPoint {
            x: LogicalPixels(geometry.x().into()),
            y: LogicalPixels(geometry.y().into()),
        };
        let size = LogicalSize {
            width: geometry.width(),
            height: geometry.height(),
        };

        let screen_id = {
            let manufacturer = monitor.manufacturer().unwrap_or_default();
            let model = monitor.model().unwrap_or_default();
            let name = monitor.connector().unwrap_or_default();
            let mut hasher = DefaultHasher::new();
            manufacturer.hash(&mut hasher);
            model.hash(&mut hasher);
            name.hash(&mut hasher);
            hasher.finish()
        };

        Self {
            screen_id: ScreenId(screen_id),
            name: monitor.connector().map_or_else(
                || RustAllocatedStrPtr::null().to_auto_drop(),
                |s| RustAllocatedStrPtr::allocate(s.as_bytes()).unwrap().to_auto_drop(),
            ),
            origin,
            size,
            scale: monitor.scale_factor().into(),
            millihertz: u32::try_from(monitor.refresh_rate()).unwrap(),
        }
    }
}

type ScreenInfoArray = AutoDropArray<ScreenInfo>;

#[unsafe(no_mangle)]
pub extern "C" fn screen_list(app_ptr: AppPtr) -> ScreenInfoArray {
    ffi_boundary("screen_list", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let display_manager = app.state.display_manager.as_ref().context("Application not yet initialized")?;
        let mut screen_infos = Vec::new();
        for display in display_manager.list_displays() {
            for monitor_obj in display.monitors().into_iter().flatten() {
                let monitor: gdk4::Monitor = monitor_obj.downcast().unwrap();
                let screen_info = ScreenInfo::new(&monitor);
                if !screen_infos.iter().any(|e: &ScreenInfo| e.screen_id == screen_info.screen_id) {
                    screen_infos.push(screen_info);
                }
            }
        }

        Ok(ScreenInfoArray::new(screen_infos.into_boxed_slice()))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn screen_list_drop(arr: ScreenInfoArray) {
    ffi_boundary("screen_list_drop", || {
        drop(arr);
        Ok(())
    });
}
