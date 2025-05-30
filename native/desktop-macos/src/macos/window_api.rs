use objc2_app_kit::{NSAppearanceCustomization, NSWindowOcclusionState};
use objc2_foundation::MainThreadMarker;

use desktop_common::{
    ffi_utils::{BorrowedStrPtr, RustAllocatedRawPtr, RustAllocatedStrPtr},
    logger::{PanicDefault, ffi_boundary},
};

use crate::geometry::{Color, LogicalPixels, LogicalPoint, LogicalRect, LogicalSize};

use super::{
    appearance::Appearance,
    application_api::MyNSApplication,
    metal_api::{MetalView, MetalViewPtr},
    screen::{NSScreenExts, ScreenId},
    string::{copy_to_c_string, copy_to_ns_string},
    text_input_client::TextInputClient,
    window::{NSWindowExts, Window},
};

pub type WindowId = isize;

pub type WindowPtr<'a> = RustAllocatedRawPtr<'a, std::ffi::c_void>;

#[repr(C)]
pub struct WindowParams<'a> {
    pub origin: LogicalPoint,
    pub size: LogicalSize,
    pub title: BorrowedStrPtr<'a>,

    pub is_resizable: bool,
    pub is_closable: bool,
    pub is_miniaturizable: bool,

    pub is_full_screen_allowed: bool,
    pub use_custom_titlebar: bool,
    pub titlebar_height: LogicalPixels,
}

#[unsafe(no_mangle)]
pub extern "C" fn window_create(params: WindowParams, text_input_client: TextInputClient) -> WindowPtr<'static> {
    let window = ffi_boundary("window_create", || {
        let mtm = MainThreadMarker::new().unwrap();
        Ok(Some(Window::new(mtm, &params, text_input_client)?))
    });
    WindowPtr::from_value(window)
}

#[unsafe(no_mangle)]
pub extern "C" fn window_drop(window_ptr: WindowPtr) {
    ffi_boundary("window_drop", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.to_owned::<Window>() };
        window.ns_window.close();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_window_id(window_ptr: WindowPtr) -> WindowId {
    ffi_boundary("window_get_window_id", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.window_id())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_screen_id(window_ptr: WindowPtr) -> ScreenId {
    ffi_boundary("window_get_screen_id", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.screen().unwrap().screen_id())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_scale_factor(window_ptr: WindowPtr) -> f64 {
    ffi_boundary("window_scale_factor", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.backingScaleFactor())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_attach_layer(window_ptr: WindowPtr, layer_ptr: MetalViewPtr) {
    ffi_boundary("window_attach_layer", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        let layer = unsafe { layer_ptr.borrow::<MetalView>() };
        window.attach_layer(layer);
        Ok(())
    });
}

impl PanicDefault for LogicalPoint {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_title(window_ptr: WindowPtr, new_title: BorrowedStrPtr) {
    ffi_boundary("window_set_title", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let new_title = copy_to_ns_string(&new_title)?;
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.setTitle(&new_title);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_title(window_ptr: WindowPtr) -> RustAllocatedStrPtr {
    ffi_boundary("window_get_title", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        let title = window.ns_window.title();
        copy_to_c_string(&title)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_origin(window_ptr: WindowPtr) -> LogicalPoint {
    ffi_boundary("window_get_origin", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.get_origin(mtm)
    })
}

impl PanicDefault for LogicalSize {
    fn default() -> Self {
        Self { width: 0.0, height: 0.0 }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_size(window_ptr: WindowPtr) -> LogicalSize {
    ffi_boundary("window_get_size", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.get_size())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_rect(window_ptr: WindowPtr, origin: LogicalPoint, size: LogicalSize, animate: bool) {
    ffi_boundary("window_set_rect", || {
        let mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.set_rect(&LogicalRect::new(origin, size), animate, mtm)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_content_origin(window_ptr: WindowPtr) -> LogicalPoint {
    ffi_boundary("window_get_content_origin", || {
        let mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.get_content_rect(mtm)?.origin)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_content_size(window_ptr: WindowPtr) -> LogicalSize {
    ffi_boundary("window_get_content_size", || {
        let mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.get_content_rect(mtm)?.size)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_content_rect(window_ptr: WindowPtr, origin: LogicalPoint, size: LogicalSize, animate: bool) {
    ffi_boundary("window_set_content_rect", || {
        let mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.set_content_rect(&LogicalRect::new(origin, size), animate, mtm)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_is_key(window_ptr: WindowPtr) -> bool {
    let window = unsafe { window_ptr.borrow::<Window>() };
    ffi_boundary("window_is_key", || Ok(window.ns_window.isKeyWindow()))
}

#[unsafe(no_mangle)]
pub extern "C" fn window_is_main(window_ptr: WindowPtr) -> bool {
    ffi_boundary("window_is_main", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        let result = unsafe { window.ns_window.isMainWindow() };
        Ok(result)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_order_front(window_ptr: WindowPtr) {
    ffi_boundary("window_order_front", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.orderFront(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_order_back(window_ptr: WindowPtr) {
    ffi_boundary("window_order_back", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        unsafe {
            window.ns_window.orderBack(None);
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_make_key_and_order_front(window_ptr: WindowPtr) {
    ffi_boundary("window_make_key_and_order_front", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.makeKeyAndOrderFront(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_max_size(window_ptr: WindowPtr) -> LogicalSize {
    ffi_boundary("window_get_max_size", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.get_max_size())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_max_size(window_ptr: WindowPtr, size: LogicalSize) {
    ffi_boundary("window_set_max_size", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.set_max_size(size);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_min_size(window_ptr: WindowPtr) -> LogicalSize {
    ffi_boundary("window_get_min_size", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.get_min_size())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_min_size(window_ptr: WindowPtr, size: LogicalSize) {
    ffi_boundary("window_set_min_size", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.set_min_size(size);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_toggle_full_screen(window_ptr: WindowPtr) {
    ffi_boundary("window_toggle_full_screen", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.toggleFullScreen(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_is_full_screen(window_ptr: WindowPtr) -> bool {
    ffi_boundary("window_is_full_screen", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.is_full_screen())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_maximize(window_ptr: WindowPtr) {
    ffi_boundary("window_maximize", || {
        // according to apple docs it shoudl be accessed only from main thread
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.zoom(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_is_maximized(window_ptr: WindowPtr) -> bool {
    ffi_boundary("window_is_maximized", || {
        // according to apple docs it shoudl be accessed only from main thread
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.isZoomed())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_miniaturize(window_ptr: WindowPtr) {
    ffi_boundary("window_miniaturize", || {
        // according to apple docs it shoudl be accessed only from main thread
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.ns_window.miniaturize(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_deminiaturize(window_ptr: WindowPtr) {
    ffi_boundary("window_deminiaturize", || {
        // according to apple docs it shoudl be accessed only from main thread
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        unsafe {
            window.ns_window.deminiaturize(None);
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_is_miniaturized(window_ptr: WindowPtr) -> bool {
    ffi_boundary("window_is_miniaturized", || {
        // according to apple docs it shoudl be accessed only from main thread
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        Ok(window.ns_window.isMiniaturized())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_is_visible(window_ptr: WindowPtr) -> bool {
    ffi_boundary("window_is_visible", || {
        let _mtm = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        let occlusion_state = window.ns_window.occlusionState();
        let is_visible = occlusion_state.contains(NSWindowOcclusionState::Visible);
        Ok(is_visible)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_drag(window_ptr: WindowPtr) {
    ffi_boundary("window_start_drag", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let app = MyNSApplication::sharedApplication(mtm);
        if let Some(event) = app.currentEvent() {
            let window = unsafe { window_ptr.borrow::<Window>() };
            window.ns_window.performWindowDragWithEvent(&event);
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_invalidate_shadow(window_ptr: WindowPtr) {
    ffi_boundary("window_invalidate_shadow", || {
        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        unsafe {
            let window = window_ptr.borrow::<Window>();
            window.ns_window.invalidateShadow();
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_appearance_override(window_ptr: WindowPtr, appearance: Appearance) {
    ffi_boundary("window_appearance_override", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        let ns_appearance = appearance.to_ns_appearance();
        unsafe {
            window.ns_window.setAppearance(Some(&ns_appearance));
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_appearacne_is_overridden(window_ptr: WindowPtr) -> bool {
    ffi_boundary("window_appearacne_is_overridden", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        let result = unsafe { window.ns_window.appearance() }.is_some();
        Ok(result)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_appearacne_set_follow_application(window_ptr: WindowPtr) {
    ffi_boundary("window_appearacne_set_follow_application", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        unsafe {
            window.ns_window.setAppearance(None);
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_appearance(window_ptr: WindowPtr) -> Appearance {
    ffi_boundary("window_get_appearance", || {
        let window = unsafe { window_ptr.borrow::<Window>() };
        let ns_appearance = unsafe { window.ns_window.effectiveAppearance() };
        let appearance = Appearance::from_ns_appearance(&ns_appearance);
        Ok(appearance)
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

#[repr(C)]
pub enum WindowBackground {
    Transparent,
    SolidColor(Color),
    VisualEffect(WindowVisualEffect),
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_background(window_ptr: WindowPtr, background: WindowBackground) {
    ffi_boundary("window_set_background", || {
        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
        let window = unsafe { window_ptr.borrow::<Window>() };
        window.set_background(mtm, background).unwrap();
        Ok(())
    });
}
