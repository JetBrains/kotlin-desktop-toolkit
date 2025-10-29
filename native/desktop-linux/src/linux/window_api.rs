use anyhow::Context;
use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;
use smithay_client_toolkit::shell::xdg::window::DecorationMode;

use super::window::SimpleWindow;
use crate::linux::{
    application::Application,
    application_api::{AppPtr, DragAndDropActions, RenderingMode},
    data_transfer::MimeTypes,
    events::{RequestId, WindowDecorationMode, WindowId},
    file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams},
    geometry::{LogicalPoint, LogicalSize},
    pointer_shapes_api::PointerShape,
    window_resize_edge_api::WindowResizeEdge,
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

    pub prefer_client_side_decoration: bool,

    pub rendering_mode: RenderingMode,
}

#[unsafe(no_mangle)]
pub extern "C" fn window_create(mut app_ptr: AppPtr, params: WindowParams) {
    ffi_boundary("window_create", || {
        debug!("window_create");

        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.new_window(&params)
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
        w.set_cursor_icon(pointer_shape);
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
    ffi_boundary("window_start_move", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.window_start_move(window_id)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_resize(app_ptr: AppPtr, window_id: WindowId, edge: WindowResizeEdge) {
    ffi_boundary("window_start_resize", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.window_start_resize(window_id, edge)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_menu(app_ptr: AppPtr, window_id: WindowId, position: LogicalPoint) {
    ffi_boundary("window_show_menu", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.window_show_menu(window_id, position)
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
pub extern "C" fn window_start_drag_and_drop(
    mut app_ptr: AppPtr,
    window_id: WindowId,
    mime_types: BorrowedStrPtr,
    actions: DragAndDropActions,
    drag_icon_rendering_mode: RenderingMode,
    drag_icon_size: LogicalSize,
) {
    debug!("application_start_drag_and_drop");
    ffi_boundary("application_start_drag_and_drop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.start_drag(
            window_id,
            MimeTypes::new(mime_types.as_str()?),
            actions.into(),
            drag_icon_rendering_mode,
            drag_icon_size,
        )
    });
}

impl From<WindowDecorationMode> for DecorationMode {
    fn from(value: WindowDecorationMode) -> Self {
        match value {
            WindowDecorationMode::Client => Self::Client,
            WindowDecorationMode::Server => Self::Server,
        }
    }
}

/// Requests the window should use the specified decoration mode.
///
/// The compositor can decide not to use the client's mode and enforce a different mode instead.
/// See <https://wayland.app/protocols/xdg-decoration-unstable-v1#zxdg_toplevel_decoration_v1:request:set_mode>
#[unsafe(no_mangle)]
pub extern "C" fn window_request_decoration_mode(app_ptr: AppPtr, window_id: WindowId, decoration_mode: WindowDecorationMode) {
    with_window(&app_ptr, window_id, "window_request_decoration_mode", |w| {
        let decoration = Some(decoration_mode.into());
        w.window.request_decoration_mode(decoration);
        Ok(())
    });
}

/// Unset the window decoration mode.
///
/// This informs the compositor that the client doesn't prefer a particular decoration mode.
/// See <https://wayland.app/protocols/xdg-decoration-unstable-v1#zxdg_toplevel_decoration_v1:request:unset_mode>
#[unsafe(no_mangle)]
pub extern "C" fn window_unset_decoration_mode(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_unset_decoration_mode", |w| {
        w.window.request_decoration_mode(None);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_open_file_dialog(
    mut app_ptr: AppPtr,
    window_id: WindowId,
    common_params: &CommonFileDialogParams,
    open_params: &OpenFileDialogParams,
) -> RequestId {
    debug!("window_show_open_file_dialog");
    ffi_boundary("window_show_open_file_dialog", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.show_open_file_dialog(window_id, common_params, open_params)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_save_file_dialog(
    mut app_ptr: AppPtr,
    window_id: WindowId,
    common_params: &CommonFileDialogParams,
    save_params: &SaveFileDialogParams,
) -> RequestId {
    debug!("window_show_save_file_dialog");
    ffi_boundary("window_show_save_file_dialog", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.show_save_file_dialog(window_id, common_params, save_params)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_activate(app_ptr: AppPtr, window_id: WindowId, token: BorrowedStrPtr) {
    ffi_boundary("application_request_activation_token", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let token_str = token.as_str()?.to_owned();
        app.window_activate(window_id, token_str)
    });
}
