use desktop_common::logger::ffi_boundary;

use super::{
    application::{AppPtr, Application},
    events::{EventHandler, LogicalPixels, LogicalSize, WindowId},
    pointer_shapes::PointerShape,
    window::WindowParams,
};

#[unsafe(no_mangle)]
pub extern "C" fn window_create(mut app_ptr: AppPtr, event_handler: EventHandler, params: WindowParams) -> WindowId {
    let window_id = ffi_boundary("window_create", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        Ok(Some(app.new_window(event_handler, &params)))
    });
    window_id.unwrap_or(WindowId(0))
}

#[unsafe(no_mangle)]
pub extern "C" fn window_drop(mut app_ptr: AppPtr, window_id: WindowId) {
    ffi_boundary("window_drop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.drop_window(window_id);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_pointer_shape(mut app_ptr: AppPtr, window_id: WindowId, pointer_shape: PointerShape) {
    ffi_boundary("window_set_pointer_shape", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        if let Some(window) = app.get_window_mut(window_id) {
            window.set_pointer_shape(pointer_shape);
        }
        Ok(())
    });
}

//#[unsafe(no_mangle)]
//pub extern "C" fn window_get_screen_id(window_ptr: WindowPtr) -> ScreenId {
//    ffi_boundary("window_get_screen_id", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        Ok(window.ns_window.screen().unwrap().screen_id())
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_scale_factor(window_ptr: WindowPtr) -> f64 {
//    ffi_boundary("window_scale_factor", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        Ok(window.ns_window.backingScaleFactor())
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_attach_layer(window_ptr: WindowPtr, layer_ptr: MetalViewPtr) {
//    ffi_boundary("window_attach_layer", || {
//        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        let layer = unsafe { layer_ptr.borrow::<MetalView>() };
//        window.attach_layer(layer);
//        Ok(())
//    });
//}
//
//impl PanicDefault for LogicalPoint {
//    fn default() -> Self {
//        Self { x: 0.0, y: 0.0 }
//    }
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_set_title(window_ptr: WindowPtr, new_title: BorrowedStrPtr) {
//    ffi_boundary("window_set_title", || {
//        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
//        let new_title = copy_to_ns_string(&new_title)?;
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        window.ns_window.setTitle(&new_title);
//        Ok(())
//    });
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_get_title(window_ptr: WindowPtr) -> RustAllocatedStrPtr {
//    ffi_boundary("window_get_title", || {
//        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        let title = window.ns_window.title();
//        copy_to_c_string(&title)
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_get_origin(window_ptr: WindowPtr) -> LogicalPoint {
//    ffi_boundary("window_get_origin", || {
//        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        window.ns_window.get_origin(mtm)
//    })
//}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_size(app_ptr: AppPtr, window_id: WindowId) -> LogicalSize {
    ffi_boundary("window_get_size", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        Ok(app.get_window_size(window_id))
    })
    .unwrap_or(LogicalSize {
        width: LogicalPixels(0.0),
        height: LogicalPixels(0.0),
    })
}

//fn with_window(app_ptr: AppPtr, window_id: WindowId, name: &str, f: impl FnOnce(&SimpleWindow)) {
//    ffi_boundary("window_set_rect", || {
//        let app = unsafe { app_ptr.borrow::<Application>() };
//        let w = app.get_window(window_id).context("No window found")?;
//        f(&w);
//        Ok(())
//    });
//}

//#[unsafe(no_mangle)]
//pub extern "C" fn window_do_frame_action(app_ptr: AppPtr, window_id: WindowId, action: FrameAction) {
//    with_window(app_ptr, window_id, "window_set_rect", |window| {
//        window.frame_action(pointer, serial, action);
//    });
//}

//#[unsafe(no_mangle)]
//pub extern "C" fn window_get_content_origin(window_ptr: WindowPtr) -> LogicalPoint {
//    ffi_boundary("window_get_content_origin", || {
//        let mtm = MainThreadMarker::new().unwrap();
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        Ok(window.ns_window.get_content_rect(mtm)?.origin)
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_get_content_size(window_ptr: WindowPtr) -> LogicalSize {
//    ffi_boundary("window_get_content_size", || {
//        let mtm = MainThreadMarker::new().unwrap();
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        Ok(window.ns_window.get_content_rect(mtm)?.size)
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_set_content_rect(window_ptr: WindowPtr, origin: LogicalPoint, size: LogicalSize, animate: bool) {
//    ffi_boundary("window_set_content_rect", || {
//        let mtm = MainThreadMarker::new().unwrap();
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        window.ns_window.set_content_rect(&LogicalRect::new(origin, size), animate, mtm)
//    });
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_is_key(window_ptr: WindowPtr) -> bool {
//    let window = unsafe { window_ptr.borrow::<Window>() };
//    ffi_boundary("window_is_key", || Ok(window.ns_window.isKeyWindow()))
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_is_main(window_ptr: WindowPtr) -> bool {
//    ffi_boundary("window_is_main", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        let result = unsafe { window.ns_window.isMainWindow() };
//        Ok(result)
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_get_max_size(window_ptr: WindowPtr) -> LogicalSize {
//    ffi_boundary("window_get_max_size", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        Ok(window.ns_window.get_max_size())
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_set_max_size(window_ptr: WindowPtr, size: LogicalSize) {
//    ffi_boundary("window_set_max_size", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        window.ns_window.set_max_size(size);
//        Ok(())
//    });
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_get_min_size(window_ptr: WindowPtr) -> LogicalSize {
//    ffi_boundary("window_get_min_size", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        Ok(window.ns_window.get_min_size())
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_set_min_size(window_ptr: WindowPtr, size: LogicalSize) {
//    ffi_boundary("window_set_min_size", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        window.ns_window.set_min_size(size);
//        Ok(())
//    });
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_toggle_full_screen(window_ptr: WindowPtr) {
//    ffi_boundary("window_toggle_full_screen", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        window.ns_window.toggleFullScreen(None);
//        Ok(())
//    });
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_is_full_screen(window_ptr: WindowPtr) -> bool {
//    ffi_boundary("window_is_full_screen", || {
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        Ok(window.ns_window.is_full_screen())
//    })
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_start_drag(window_ptr: WindowPtr) {
//    ffi_boundary("window_start_drag", || {
//        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
//        let app = MyNSApplication::sharedApplication(mtm);
//        if let Some(event) = app.currentEvent() {
//            let window = unsafe { window_ptr.borrow::<Window>() };
//            window.ns_window.performWindowDragWithEvent(&event);
//        }
//        Ok(())
//    });
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_invalidate_shadow(window_ptr: WindowPtr) {
//    ffi_boundary("window_invalidate_shadow", || {
//        let _mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
//        unsafe {
//            let window = window_ptr.borrow::<Window>();
//            window.ns_window.invalidateShadow();
//        }
//        Ok(())
//    });
//}
//
//#[derive(Debug)]
//#[repr(C)]
//pub enum WindowVisualEffect {
//    TitlebarEffect,
//    SelectionEffect,
//    MenuEffect,
//    PopoverEffect,
//    SidebarEffect,
//    HeaderViewEffect,
//    SheetEffect,
//    WindowBackgroundEffect,
//    HUDWindowEffect,
//    FullScreenUIEffect,
//    ToolTipEffect,
//    ContentBackgroundEffect,
//    UnderWindowBackgroundEffect,
//    UnderPageBackgroundEffect,
//}
//
//#[repr(C)]
//pub enum WindowBackground {
//    Transparent,
//    SolidColor(Color),
//    VisualEffect(WindowVisualEffect),
//}
//
//#[unsafe(no_mangle)]
//pub extern "C" fn window_set_background(window_ptr: WindowPtr, background: WindowBackground) {
//    ffi_boundary("window_set_background", || {
//        let mtm: MainThreadMarker = MainThreadMarker::new().unwrap();
//        let window = unsafe { window_ptr.borrow::<Window>() };
//        window.set_background(mtm, background).unwrap();
//        Ok(())
//    });
//}
