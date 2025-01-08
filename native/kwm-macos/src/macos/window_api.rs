use objc2_foundation::MainThreadMarker;

use crate::common::{Color, LogicalPixels, LogicalPoint, LogicalSize, StrPtr};

use super::{application_api::MyNSApplication, metal_api::MetalView, screen::{NSScreenExts, ScreenId}, window::{NSWindowExts, Window}};

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

#[no_mangle]
pub extern "C" fn window_create(params: &WindowParams) -> Box<Window> {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let window = Window::new(mtm, params);
    return Box::new(window)
}

#[no_mangle]
pub extern "C" fn window_drop(window: Box<Window>) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    window.ns_window.close();
    std::mem::drop(window);
}

#[no_mangle]
pub extern "C" fn window_get_window_id(window: &Window) -> WindowId {
    return window.ns_window.window_id();
}

#[no_mangle]
pub extern "C" fn window_get_screen_id(window: &Window) -> ScreenId {
    return window.ns_window.screen().unwrap().screen_id();
}

#[no_mangle]
pub extern "C" fn window_scale_factor(window: &Window) -> f64 {
    return window.ns_window.backingScaleFactor();
}

#[no_mangle]
pub extern "C" fn window_attach_layer(window: &Window, layer: &MetalView) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    window.attach_layer(layer);
}

#[no_mangle]
pub extern "C" fn window_get_origin(window: &Window) -> LogicalPoint {
    return window.ns_window.get_origin()
}

#[no_mangle]
pub extern "C" fn window_get_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_size()
}

#[no_mangle]
pub extern "C" fn window_set_rect(window: &Window, origin: LogicalPoint, size: LogicalSize, animate: bool) {
    window.ns_window.set_rect(origin.into(), size.into(), animate);
}

#[no_mangle]
pub extern "C" fn window_is_key(window: &Window) -> bool {
    return window.ns_window.isKeyWindow();
}

#[no_mangle]
pub extern "C" fn window_is_main(window: &Window) -> bool {
    return unsafe {
        window.ns_window.isMainWindow()
    }
}

#[no_mangle]
pub extern "C" fn window_get_max_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_max_size();
}

#[no_mangle]
pub extern "C" fn window_set_max_size(window: &Window, size: LogicalSize) {
    window.ns_window.set_max_size(size);
}

#[no_mangle]
pub extern "C" fn window_get_min_size(window: &Window) -> LogicalSize {
    return window.ns_window.get_min_size();
}

#[no_mangle]
pub extern "C" fn window_set_min_size(window: &Window, size: LogicalSize) {
    window.ns_window.set_min_size(size);
}

#[no_mangle]
pub extern "C" fn window_toggle_full_screen(window: &Window) {
    window.ns_window.toggleFullScreen(None);
}

#[no_mangle]
pub extern "C" fn window_is_full_screen(window: &Window) -> bool {
    return window.ns_window.is_full_screen();
}

#[no_mangle]
pub extern "C" fn window_start_drag(window: &Window) {
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    let app = MyNSApplication::sharedApplication(mtm);
    if let Some(event) = app.currentEvent() {
        window.ns_window.performWindowDragWithEvent(&event);
    }
}

#[no_mangle]
pub extern "C" fn window_invalidate_shadow(window: &Window) {
    let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    unsafe {
        window.ns_window.invalidateShadow();
    }
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
    let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
    window.set_background(mtm, background).unwrap();
}