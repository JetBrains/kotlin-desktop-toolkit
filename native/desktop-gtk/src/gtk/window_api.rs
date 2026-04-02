use super::window::SimpleWindow;
use crate::gtk::application::with_app_state;
use crate::gtk::application_api::RenderingMode;
use crate::gtk::data_transfer_api::DragAndDropActions;
use crate::gtk::events::{RequestId, WindowDecorationMode, WindowId};
use crate::gtk::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use crate::gtk::geometry::LogicalSize;
use crate::gtk::mime_types::MimeTypes;
use crate::gtk::pointer_shapes_api::PointerShape;
use crate::gtk::text_input_api::TextInputContext;

use desktop_common::{
    ffi_utils::BorrowedStrPtr,
    logger::{PanicDefault, ffi_boundary},
};
use log::debug;

fn with_window(window_id: WindowId, name: &str, f: impl FnOnce(&SimpleWindow) -> anyhow::Result<()>) {
    ffi_boundary(name, || with_app_state(|app| app.with_window(window_id, f)));
}

#[repr(C)]
pub struct WindowParams<'a> {
    pub window_id: WindowId,

    pub size: LogicalSize,

    pub min_size: LogicalSize,

    pub title: BorrowedStrPtr<'a>,

    pub decoration_mode: WindowDecorationMode,

    pub rendering_mode: RenderingMode,
}

#[unsafe(no_mangle)]
pub extern "C" fn window_create(params: WindowParams) {
    debug!("window_create");
    ffi_boundary("window_create", || {
        debug!("window_create; {:?}", params.decoration_mode);

        with_app_state(|app| app.new_window(&params))
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_close(window_id: WindowId) {
    debug!("window_close: {window_id:?}");
    ffi_boundary("window_close", || {
        with_app_state(|app| {
            app.close_window(window_id);
            Ok(())
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_request_redraw(window_id: WindowId) {
    with_window(window_id, "window_request_redraw", |w| {
        w.request_redraw();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_pointer_shape(window_id: WindowId, pointer_shape: PointerShape) {
    debug!("window_set_pointer_shape for {window_id:?}: {pointer_shape:?}");
    with_window(window_id, "window_set_pointer_shape", |w| {
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
pub extern "C" fn window_set_title(window_id: WindowId, new_title: BorrowedStrPtr) {
    with_window(window_id, "window_set_title", |w| {
        w.set_title(new_title.as_str()?);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_maximize(window_id: WindowId) {
    with_window(window_id, "window_maximize", |w| {
        w.maximize();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_unmaximize(window_id: WindowId) {
    with_window(window_id, "window_maximize", |w| {
        w.unmaximize();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_minimize(window_id: WindowId) {
    with_window(window_id, "window_minimize", |w| {
        w.minimize();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_min_size(window_id: WindowId, size: LogicalSize) {
    with_window(window_id, "window_set_min_size", |w| {
        w.set_min_size(size);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_set_fullscreen(window_id: WindowId) {
    debug!("window_set_fullscreen for {window_id:?}");
    with_window(window_id, "window_set_fullscreen", |w| {
        w.set_fullscreen();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_unset_fullscreen(window_id: WindowId) {
    debug!("window_unset_fullscreen for {window_id:?}");
    with_window(window_id, "window_unset_fullscreen", |w| {
        w.unset_fullscreen();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_start_drag_and_drop(
    window_id: WindowId,
    mime_types: BorrowedStrPtr,
    actions: DragAndDropActions,
    drag_icon_rendering_mode: RenderingMode,
    drag_icon_size: LogicalSize,
) {
    debug!("window_start_drag_and_drop");
    ffi_boundary("window_start_drag_and_drop", || {
        with_app_state(|app| {
            app.start_drag(
                window_id,
                &MimeTypes::new(mime_types.as_str()?),
                actions.into(),
                drag_icon_rendering_mode,
                drag_icon_size,
            )
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_open_file_dialog(
    window_id: WindowId,
    common_params: &CommonFileDialogParams,
    open_params: &OpenFileDialogParams,
) -> RequestId {
    debug!("window_show_open_file_dialog");
    ffi_boundary("window_show_open_file_dialog", || {
        with_app_state(|app| app.show_open_file_dialog(window_id, common_params, open_params))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_show_save_file_dialog(
    window_id: WindowId,
    common_params: &CommonFileDialogParams,
    save_params: &SaveFileDialogParams,
) -> RequestId {
    debug!("window_show_save_file_dialog");
    ffi_boundary("window_show_save_file_dialog", || {
        with_app_state(|app| app.show_save_file_dialog(window_id, common_params, save_params))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn window_activate(window_id: WindowId, token: BorrowedStrPtr) {
    debug!("window_activate: {window_id:?}");
    with_window(window_id, "window_activate", |w| {
        let token_str = token.as_optional_str()?;
        w.focus(token_str);
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn window_text_input_enable(window_id: WindowId, context: TextInputContext) {
    with_window(window_id, "window_text_input_enable", |w| w.text_input_enable(&context));
}

#[unsafe(no_mangle)]
pub extern "C" fn window_text_input_update(window_id: WindowId, context: TextInputContext) {
    with_window(window_id, "application_text_input_update", |w| w.text_input_update(&context));
}

#[unsafe(no_mangle)]
pub extern "C" fn window_text_input_disable(window_id: WindowId) {
    with_window(window_id, "window_text_input_disable", |w| {
        w.text_input_disable();
        Ok(())
    });
}
