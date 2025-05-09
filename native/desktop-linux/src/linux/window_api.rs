use super::{
    application::Application,
    application_api::AppPtr,
    events::WindowId,
    geometry::{LogicalPoint, LogicalSize},
    pointer_shapes::PointerShape,
    window_resize_edge::WindowResizeEdge,
};
use crate::linux::clipboard::MimeTypes;
use anyhow::Context;
use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;
use smithay_client_toolkit::reexports::client::protocol::wl_data_device_manager::DndAction;
use smithay_client_toolkit::shell::xdg::window::Window;

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
    ffi_boundary("window_close", || {
        debug!("window_close");
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let w = app.get_window_mut(window_id).context("No window found")?;
        w.close();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_pointer_shape(mut app_ptr: AppPtr, window_id: WindowId, pointer_shape: PointerShape) {
    ffi_boundary("window_set_pointer_shape", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        if let Some(window) = app.get_window_mut(window_id) {
            window.set_cursor_icon(pointer_shape.into());
        }
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_get_size(app_ptr: AppPtr, window_id: WindowId) -> LogicalSize {
    ffi_boundary("window_get_size", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        Ok(app.get_window_size(window_id))
    })
    .unwrap_or_default()
}

#[unsafe(no_mangle)]
pub extern "C" fn window_is_key(app_ptr: AppPtr, _window_id: WindowId) -> bool {
    ffi_boundary("window_is_key", || {
        let _app = unsafe { app_ptr.borrow::<Application>() };
        //Ok(app.window_is_key(window_id))
        Ok(Some(true))
    })
    .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn window_is_main(app_ptr: AppPtr, _window_id: WindowId) -> bool {
    ffi_boundary("window_is_main", || {
        let _app = unsafe { app_ptr.borrow::<Application>() };
        //Ok(app.window_is_main(window_id))
        Ok(Some(true))
    })
    .unwrap_or(false)
}

fn with_window<R: PanicDefault>(app_ptr: &AppPtr, window_id: WindowId, name: &str, f: impl FnOnce(&Window) -> anyhow::Result<R>) {
    ffi_boundary(name, || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let w = app
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        f(&w.window)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_title(app_ptr: AppPtr, window_id: WindowId, new_title: BorrowedStrPtr) {
    with_window(&app_ptr, window_id, "window_set_title", |w| {
        w.set_title(new_title.as_str()?);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_move(app_ptr: AppPtr, window_id: WindowId) {
    ffi_boundary("window_start_move", || {
        debug!("window_start_move");
        let app = unsafe { app_ptr.borrow::<Application>() };
        let w = app
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        w.start_move();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_resize(app_ptr: AppPtr, window_id: WindowId, edge: WindowResizeEdge) {
    ffi_boundary("window_start_resize", || {
        debug!("window_start_resize");
        let app = unsafe { app_ptr.borrow::<Application>() };
        let w = app
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        w.start_resize(edge);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_menu(app_ptr: AppPtr, window_id: WindowId, position: LogicalPoint) {
    ffi_boundary("window_show_menu", || {
        debug!("window_show_menu");
        let app = unsafe { app_ptr.borrow::<Application>() };
        let w = app.get_window(window_id).context("No window found")?;
        w.show_menu(position);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_maximize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_maximize", |w| {
        w.set_maximized();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_unmaximize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_maximize", |w| {
        w.unset_maximized();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_minimize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_minimize", |w| {
        w.set_minimized();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_max_size(app_ptr: AppPtr, window_id: WindowId, size: LogicalSize) {
    with_window(&app_ptr, window_id, "window_set_max_size", |w| {
        #[allow(clippy::cast_sign_loss)]
        w.set_max_size(Some((size.width.round() as u32, size.height.round() as u32))); // TODO: check
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_min_size(app_ptr: AppPtr, window_id: WindowId, size: LogicalSize) {
    with_window(&app_ptr, window_id, "window_set_min_size", |w| {
        #[allow(clippy::cast_sign_loss)]
        w.set_min_size(Some((size.width.round() as u32, size.height.round() as u32))); // TODO: check
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_fullscreen(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_toggle_full_screen", |w| {
        w.set_fullscreen(None /* output, let the compositor choose */);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_unset_fullscreen(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_toggle_full_screen", |w| {
        w.unset_fullscreen();
        Ok(())
    });
}

#[repr(C)]
pub enum DragAction {
    Copy,
    Move,
    Ask,
}

impl From<DragAction> for DndAction {
    fn from(value: DragAction) -> Self {
        match value {
            DragAction::Copy => Self::Copy,
            DragAction::Move => Self::Move,
            DragAction::Ask => Self::Ask,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_drag(mut app_ptr: AppPtr, window_id: WindowId, mime_types: BorrowedStrPtr, action: DragAction) {
    ffi_boundary("window_show_menu", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.start_drag(window_id, MimeTypes::new(mime_types.as_str()?), action.into())
    });
}
