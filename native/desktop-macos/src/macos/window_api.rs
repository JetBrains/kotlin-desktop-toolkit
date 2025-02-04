use core::{panic};
use std::{ffi::{CStr, CString}, panic::{AssertUnwindSafe, UnwindSafe}};

use objc2::{exception::{self, Exception}, rc::{autoreleasepool, Retained}};
use objc2_foundation::{MainThreadMarker, NSException, NSString};

use crate::{common::{Color, LogicalPixels, LogicalPoint, LogicalRect, LogicalSize, StrPtr}, logger::{ffi_boundary, PanicDefault}};

use super::{application_api::MyNSApplication, metal_api::MetalView, screen::{NSScreenExts, ScreenId}, string::{copy_to_c_string, copy_to_ns_string}, window::{NSWindowExts, Window}};

pub type WindowId = i64;

#[repr(C)]
pub struct WindowParams {
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub title: StrPtr,

    pub is_resizable: bool,
    pub is_closable: bool,
    pub is_miniaturizable: bool,

    pub is_full_screen_allowed: bool,
    pub use_custom_titlebar: bool,
    pub titlebar_height: LogicalPixels
}

impl PanicDefault for *mut Window {
    fn default() -> Self {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn window_create(params: &WindowParams) -> *mut Window {
    ffi_boundary("window_create", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let window = Window::new(mtm, params)?;
        Ok(Box::into_raw(Box::new(window)))
    })
}

#[no_mangle]
pub extern "C" fn window_drop(window: *mut Window) {
    ffi_boundary("window_drop", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let window: Box<Window> = unsafe {
            assert!(!window.is_null());
            Box::from_raw(window)
        };
        window.ns_window.close();
        std::mem::drop(window);
        Ok(())
    })
}

impl PanicDefault for WindowId {
    fn default() -> Self {
        0
    }
}

#[no_mangle]
pub extern "C" fn window_get_window_id(window: &Window) -> WindowId {
    ffi_boundary("window_get_window_id", || {
        Ok(window.ns_window.window_id())
    })
}

impl PanicDefault for ScreenId {
    fn default() -> Self {
        0
    }
}

#[no_mangle]
pub extern "C" fn window_get_screen_id(window: &Window) -> ScreenId {
    ffi_boundary("window_get_screen_id", || {
        Ok(window.ns_window.screen().unwrap().screen_id())
    })
}

impl PanicDefault for f64 {
    fn default() -> Self {
        0.0
    }
}

#[no_mangle]
pub extern "C" fn window_scale_factor(window: &Window) -> f64 {
    ffi_boundary("window_scale_factor", || {
        Ok(window.ns_window.backingScaleFactor())
    })
}

#[no_mangle]
pub extern "C" fn window_attach_layer(window: &Window, layer: &MetalView) {
    ffi_boundary("window_attach_layer", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        window.attach_layer(layer);
        Ok(())
    })
}

impl PanicDefault for LogicalPoint {
    fn default() -> Self {
        LogicalPoint {
            x: 0.0,
            y: 0.0,
        }
    }
}

#[no_mangle]
pub extern "C" fn window_set_title(window: &Window, new_title: StrPtr) {
    ffi_boundary("window_set_title", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let new_title = copy_to_ns_string(new_title)?;
        window.ns_window.setTitle(&new_title);
        Ok(())
    })
}

impl PanicDefault for StrPtr {
    fn default() -> Self {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn window_get_title(window: &Window) -> StrPtr {
    ffi_boundary("window_get_title", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let title = window.ns_window.title();
        autoreleasepool(|pool| {
            copy_to_c_string(&title, pool)
        })
    })
}

#[no_mangle]
pub extern "C" fn window_get_origin(window: &Window) -> LogicalPoint {
    ffi_boundary("window_get_origin", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        window.ns_window.get_origin(mtm)
    })
}

impl PanicDefault for LogicalSize {
    fn default() -> Self {
        LogicalSize {
            width: 0.0,
            height: 0.0,
        }
    }
}

#[no_mangle]
pub extern "C" fn window_get_size(window: &Window) -> LogicalSize {
    ffi_boundary("window_get_size", || {
        Ok(window.ns_window.get_size())
    })
}

#[no_mangle]
pub extern "C" fn window_set_rect(window: &Window, origin: LogicalPoint, size: LogicalSize, animate: bool) {
    ffi_boundary("window_set_rect", || {
        let mtm = MainThreadMarker::new().unwrap();
        window.ns_window.set_rect(&LogicalRect::new(origin, size), animate, mtm)
    })
}

#[no_mangle]
pub extern "C" fn window_is_key(window: &Window) -> bool {
    ffi_boundary("window_is_key", || {
        Ok(window.ns_window.isKeyWindow())
    })
}

#[no_mangle]
pub extern "C" fn window_is_main(window: &Window) -> bool {
    ffi_boundary("window_is_main", || {
        let result = unsafe {
            window.ns_window.isMainWindow()
        };
        Ok(result)
    })
}

#[no_mangle]
pub extern "C" fn window_get_max_size(window: &Window) -> LogicalSize {
    ffi_boundary("window_get_max_size", || {
        Ok(window.ns_window.get_max_size())
    })
}

#[no_mangle]
pub extern "C" fn window_set_max_size(window: &Window, size: LogicalSize) {
    ffi_boundary("window_set_max_size", || {
        window.ns_window.set_max_size(size);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn window_get_min_size(window: &Window) -> LogicalSize {
    ffi_boundary("window_get_min_size", || {
        Ok(window.ns_window.get_min_size())
    })
}

#[no_mangle]
pub extern "C" fn window_set_min_size(window: &Window, size: LogicalSize) {
    ffi_boundary("window_set_min_size", || {
        window.ns_window.set_min_size(size);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn window_toggle_full_screen(window: &Window) {
    ffi_boundary("window_toggle_full_screen", || {
        window.ns_window.toggleFullScreen(None);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn window_is_full_screen(window: &Window) -> bool {
    ffi_boundary("window_is_full_screen", || {
        Ok(window.ns_window.is_full_screen())
    })
}

#[no_mangle]
pub extern "C" fn window_start_drag(window: &Window) {
    ffi_boundary("window_start_drag", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        if let Some(event) = app.currentEvent() {
            window.ns_window.performWindowDragWithEvent(&event);
        }
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn window_invalidate_shadow(window: &Window) {
    ffi_boundary("window_invalidate_shadow", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        unsafe {
            window.ns_window.invalidateShadow();
        }
        Ok(())
    })
}

#[derive(Debug)]
#[repr(C)]
pub enum WindowVisualEffect {
    TitlebarEffect,
    SelectionEffect,
    MenuEffect,
    PopoverEffect,
    SidebarEffect,
    HeaderViewEffect,
    SheetEffect,
    WindowBackgroundEffect,
    HUDWindowEffect,
    FullScreenUIEffect,
    ToolTipEffect,
    ContentBackgroundEffect,
    UnderWindowBackgroundEffect,
    UnderPageBackgroundEffect,
}

#[allow(dead_code)]
#[repr(C)]
pub enum WindowBackground {
    Transparent,
    SolidColor(Color),
    VisualEffect(WindowVisualEffect)
}

#[no_mangle]
pub extern "C" fn window_set_background(window: &Window, background: WindowBackground) {
    ffi_boundary("window_set_background", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        window.set_background(mtm, background).unwrap();
        Ok(())
    })
}