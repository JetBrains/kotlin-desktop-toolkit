use anyhow::Context;
use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;

use super::window::SimpleWindow;
use crate::linux::{
    application::Application,
    application_api::AppPtr,
    events::WindowId,
    geometry::{LogicalPoint, LogicalSize},
    pointer_shapes::PointerShape,
    window_resize_edge::WindowResizeEdge,
};

fn with_window<R: PanicDefault>(
    app_ptr: &AppPtr,
    window_id: WindowId,
    name: &str,
    f: impl FnOnce(&SimpleWindow) -> anyhow::Result<R>,
) -> R {
    ffi_boundary(name, || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let w = app
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        f(w)
    })
}

fn with_window_mut<R: PanicDefault>(
    app_ptr: &mut AppPtr,
    window_id: WindowId,
    name: &str,
    f: impl FnOnce(&mut SimpleWindow) -> anyhow::Result<R>,
) -> R {
    ffi_boundary(name, || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let w = app
            .get_window_mut(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        f(w)
    })
}

#[repr(C)]
pub struct WindowParams<'a> {
    pub window_id: WindowId,

    pub size: LogicalSize,

    pub title: BorrowedStrPtr<'a>,

    /// See <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_app_id>
    pub app_id: BorrowedStrPtr<'a>,

    pub force_client_side_decoration: bool,

    pub force_software_rendering: bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn window_create(mut app_ptr: AppPtr, params: WindowParams) {
    ffi_boundary("window_create", || {
        debug!("window_create");

        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.new_window(&params);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_close(mut app_ptr: AppPtr, window_id: WindowId) {
    with_window_mut(&mut app_ptr, window_id, "window_close", |w| {
        w.close();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_pointer_shape(mut app_ptr: AppPtr, window_id: WindowId, pointer_shape: PointerShape) {
    with_window_mut(&mut app_ptr, window_id, "window_set_pointer_shape", |w| {
        w.set_cursor_icon(pointer_shape.into());
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_size(app_ptr: AppPtr, window_id: WindowId) -> LogicalSize {
    with_window(&app_ptr, window_id, "window_get_size", |w| Ok(w.size)).unwrap_or_default()
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_title(app_ptr: AppPtr, window_id: WindowId, new_title: BorrowedStrPtr) {
    with_window(&app_ptr, window_id, "window_set_title", |w| {
        w.window.set_title(new_title.as_str()?);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_move(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_start_move", |w| {
        w.start_move();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_resize(app_ptr: AppPtr, window_id: WindowId, edge: WindowResizeEdge) {
    with_window(&app_ptr, window_id, "window_start_resize", |w| {
        w.start_resize(edge);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_menu(app_ptr: AppPtr, window_id: WindowId, position: LogicalPoint) {
    with_window(&app_ptr, window_id, "window_show_menu", |w| {
        w.show_menu(position);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_maximize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_maximize", |w| {
        w.window.set_maximized();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_unmaximize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_maximize", |w| {
        w.window.unset_maximized();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_minimize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_minimize", |w| {
        w.window.set_minimized();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_max_size(app_ptr: AppPtr, window_id: WindowId, size: LogicalSize) {
    with_window(&app_ptr, window_id, "window_set_max_size", |w| {
        #[allow(clippy::cast_sign_loss)]
        w.window.set_max_size(Some((size.width.round() as u32, size.height.round() as u32)));
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_min_size(app_ptr: AppPtr, window_id: WindowId, size: LogicalSize) {
    with_window(&app_ptr, window_id, "window_set_min_size", |w| {
        #[allow(clippy::cast_sign_loss)]
        w.window.set_min_size(Some((size.width.round() as u32, size.height.round() as u32)));
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_fullscreen(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_toggle_full_screen", |w| {
        w.window.set_fullscreen(None /* output, let the compositor choose */);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_unset_fullscreen(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_toggle_full_screen", |w| {
        w.window.unset_fullscreen();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_clipboard_paste(
    app_ptr: AppPtr<'_>,
    window_id: WindowId,
    serial: i32,
    supported_mime_types: BorrowedStrPtr,
) -> bool {
    let t = std::thread::current();
    debug!("window_clipboard_paste, thread id: {:?} ({:?})", t.id(), t.name());
    ffi_boundary("window_clipboard_paste", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.clipboard_paste(window_id, serial, supported_mime_types.as_str()?)
    })
}
