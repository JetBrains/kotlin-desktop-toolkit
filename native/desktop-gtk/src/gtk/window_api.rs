use super::window::SimpleWindow;
use crate::gtk::application::Application;
use crate::gtk::application_api::{AppPtr, DragAndDropActions, RenderingMode};
use crate::gtk::events::{RequestId, WindowDecorationMode, WindowId};
use crate::gtk::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use crate::gtk::geometry::LogicalSize;
use crate::gtk::mime_types::MimeTypes;
use crate::gtk::pointer_shapes_api::PointerShape;
use crate::gtk::text_input_api::TextInputContext;
use crate::gtk::window_resize_edge_api::WindowResizeEdge;

use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;

fn with_window(app_ptr: &AppPtr, window_id: WindowId, name: &str, f: impl FnOnce(&SimpleWindow) -> anyhow::Result<()>) {
    ffi_boundary(name, || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.with_window(window_id, f)
    });
}

#[repr(C)]
pub struct WindowParams<'a> {
    pub window_id: WindowId,

    pub size: LogicalSize,

    pub min_size: LogicalSize,

    pub title: BorrowedStrPtr<'a>,

    pub prefer_client_side_decoration: bool,

    pub rendering_mode: RenderingMode,
}

#[unsafe(no_mangle)]
pub extern "C" fn window_create(app_ptr: AppPtr, params: WindowParams) {
    debug!("window_create");
    ffi_boundary("window_create", || {
        debug!(
            "window_create; prefer_client_side_decoration={}",
            params.prefer_client_side_decoration
        );

        let app = unsafe { app_ptr.borrow::<Application>() };
        app.new_window(&params)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_close(app_ptr: AppPtr, window_id: WindowId) {
    debug!("window_close: {window_id:?}");
    ffi_boundary("window_close", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.close_window(window_id);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_request_redraw(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_request_redraw", |w| {
        w.request_redraw();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_pointer_shape(app_ptr: AppPtr, window_id: WindowId, pointer_shape: PointerShape) {
    with_window(&app_ptr, window_id, "window_set_pointer_shape", |w| {
        w.set_cursor_icon(pointer_shape);
        Ok(())
    });
}

impl PanicDefault for LogicalSize {
    fn default() -> Self {
        <Self as Default>::default()
    }
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
    debug!("window_start_move for {window_id:?}");
    with_window(&app_ptr, window_id, "window_start_move", SimpleWindow::start_move);
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_resize(app_ptr: AppPtr, window_id: WindowId, edge: WindowResizeEdge) {
    debug!("window_start_resize for {window_id:?}");
    with_window(&app_ptr, window_id, "window_start_resize", |w| w.start_resize(edge));
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_menu(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_show_menu", SimpleWindow::show_menu);
}

#[unsafe(no_mangle)]
pub extern "C" fn window_maximize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_maximize", |w| {
        w.maximize();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_unmaximize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_maximize", |w| {
        w.unmaximize();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_minimize(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_minimize", |w| {
        w.minimize();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_min_size(app_ptr: AppPtr, window_id: WindowId, size: LogicalSize) {
    with_window(&app_ptr, window_id, "window_set_min_size", |w| {
        w.set_min_size(size);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_fullscreen(app_ptr: AppPtr, window_id: WindowId) {
    debug!("window_set_fullscreen for {window_id:?}");
    with_window(&app_ptr, window_id, "window_set_fullscreen", |w| {
        w.set_fullscreen();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_unset_fullscreen(app_ptr: AppPtr, window_id: WindowId) {
    debug!("window_unset_fullscreen for {window_id:?}");
    with_window(&app_ptr, window_id, "window_unset_fullscreen", |w| {
        w.unset_fullscreen();
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
    debug!("window_start_drag_and_drop");
    _ = drag_icon_rendering_mode;
    ffi_boundary("window_start_drag_and_drop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.start_drag(window_id, &MimeTypes::new(mime_types.as_str()?), actions.into(), drag_icon_size)
    });
}

/// Requests the window should use the specified decoration mode.
#[unsafe(no_mangle)]
pub extern "C" fn window_request_decoration_mode(app_ptr: AppPtr, window_id: WindowId, decoration_mode: WindowDecorationMode) {
    debug!("window_request_decoration_mode {decoration_mode:?} for {window_id:?}");
    with_window(&app_ptr, window_id, "window_request_decoration_mode", |w| {
        w.set_decoration_mode(decoration_mode);
        Ok(())
    });
}

/// Unset the window decoration mode.
///
/// This informs the compositor that the client doesn't prefer a particular decoration mode.
/// See <https://wayland.app/protocols/xdg-decoration-unstable-v1#zxdg_toplevel_decoration_v1:request:unset_mode>
#[unsafe(no_mangle)]
pub extern "C" fn window_unset_decoration_mode(app_ptr: AppPtr, window_id: WindowId) {
    debug!("window_unset_decoration_mode for {window_id:?}");
    with_window(&app_ptr, window_id, "window_unset_decoration_mode", |w| {
        w.set_decoration_mode(WindowDecorationMode::Server);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_open_file_dialog(
    app_ptr: AppPtr,
    window_id: WindowId,
    common_params: &CommonFileDialogParams,
    open_params: &OpenFileDialogParams,
) -> RequestId {
    debug!("window_show_open_file_dialog");
    ffi_boundary("window_show_open_file_dialog", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.show_open_file_dialog(window_id, common_params, open_params)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_save_file_dialog(
    app_ptr: AppPtr,
    window_id: WindowId,
    common_params: &CommonFileDialogParams,
    save_params: &SaveFileDialogParams,
) -> RequestId {
    debug!("window_show_save_file_dialog");
    ffi_boundary("window_show_save_file_dialog", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.show_save_file_dialog(window_id, common_params, save_params)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_activate(app_ptr: AppPtr, window_id: WindowId, token: BorrowedStrPtr) {
    debug!("window_activate: {window_id:?}");
    with_window(&app_ptr, window_id, "window_activate", |w| {
        let token_str = token.as_optional_str()?;
        w.focus(token_str);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_text_input_enable(app_ptr: AppPtr, window_id: WindowId, context: TextInputContext) {
    with_window(&app_ptr, window_id, "window_text_input_enable", |w| w.text_input_enable(&context));
}

#[unsafe(no_mangle)]
pub extern "C" fn window_text_input_update(app_ptr: AppPtr, window_id: WindowId, context: TextInputContext) {
    with_window(&app_ptr, window_id, "application_text_input_update", |w| {
        w.text_input_update(&context)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_text_input_disable(app_ptr: AppPtr, window_id: WindowId) {
    with_window(&app_ptr, window_id, "window_text_input_disable", |w| {
        w.text_input_disable();
        Ok(())
    });
}
