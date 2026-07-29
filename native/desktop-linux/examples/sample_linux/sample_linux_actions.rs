use desktop_common::ffi_utils::BorrowedUtf8;
use desktop_linux::linux::application_api::AppPtr;
use desktop_linux::linux::events::WindowFrame;
use desktop_linux::linux::window_api::{window_minimize, window_set_client_side_decoration_frame};
use desktop_linux::linux::{
    application_api::{
        RenderingMode,
        application_clipboard_paste,
        application_clipboard_put,
        application_primary_selection_paste,
        application_primary_selection_put,
        application_stop_event_loop,
        //
    },
    events::WindowId,
    geometry::LogicalSize,
    window_api::{
        WindowParams,
        window_activate,
        window_close,
        window_create,
        window_maximize,
        window_set_fullscreen,
        window_unmaximize,
        window_unset_fullscreen,
        //
    },
};

#[allow(dead_code)]
pub enum Action {
    Dummy,
    ApplicationClipboardPaste {
        serial: i32,
        supported_mime_types: &'static str,
    },
    ApplicationClipboardPut(&'static str),
    ApplicationPrimarySelectionPaste {
        serial: i32,
        supported_mime_types: &'static str,
    },
    ApplicationPrimarySelectionPut(&'static str),
    ApplicationStopEventLoop,
    WindowActivate {
        window_id: WindowId,
        token: Option<String>,
    },
    WindowCreate {
        window_id: WindowId,
        size: LogicalSize,
        min_size: LogicalSize,
        app_id: String,
        title: String,
        prefer_client_side_decoration: bool,
        rendering_mode: RenderingMode,
        client_side_decoration_frame: WindowFrame,
    },
    WindowClose(WindowId),
    WindowMaximize(WindowId),
    WindowMinimize(WindowId),
    WindowSetClientFrame {
        window_id: WindowId,
        frame: WindowFrame,
    },
    WindowSetFullscreen(WindowId),
    WindowUnmaximize(WindowId),
    WindowUnsetFullscreen(WindowId),
}

impl Action {
    pub fn perform(self, app_ptr: AppPtr) {
        match self {
            Self::Dummy => {}
            Self::ApplicationStopEventLoop => application_stop_event_loop(app_ptr),
            Self::WindowActivate { window_id, token } => window_activate(app_ptr, window_id, BorrowedUtf8::optional(token.as_ref())),
            Self::WindowCreate {
                window_id,
                size,
                min_size,
                app_id,
                title,
                prefer_client_side_decoration,
                rendering_mode,
                client_side_decoration_frame,
            } => {
                window_create(
                    app_ptr,
                    WindowParams {
                        window_id,
                        size,
                        min_size,
                        title: BorrowedUtf8::new(&title),
                        app_id: BorrowedUtf8::new(&app_id),
                        rendering_mode,
                        prefer_client_side_decoration,
                        client_side_decoration_frame,
                    },
                );
            }
            Self::WindowClose(window_id) => window_close(app_ptr, window_id),
            Self::WindowMaximize(window_id) => window_maximize(app_ptr, window_id),
            Self::WindowMinimize(window_id) => window_minimize(app_ptr, window_id),
            Self::WindowSetFullscreen(window_id) => window_set_fullscreen(app_ptr, window_id),
            Self::WindowUnmaximize(window_id) => window_unmaximize(app_ptr, window_id),
            Self::WindowUnsetFullscreen(window_id) => window_unset_fullscreen(app_ptr, window_id),
            Self::ApplicationClipboardPaste {
                serial,
                supported_mime_types,
            } => application_clipboard_paste(app_ptr, serial, BorrowedUtf8::new(supported_mime_types)),
            Self::ApplicationClipboardPut(mime_types) => application_clipboard_put(app_ptr, BorrowedUtf8::new(mime_types)),
            Self::ApplicationPrimarySelectionPaste {
                serial,
                supported_mime_types,
            } => application_primary_selection_paste(app_ptr, serial, BorrowedUtf8::new(supported_mime_types)),
            Self::ApplicationPrimarySelectionPut(mime_types) => application_primary_selection_put(app_ptr, BorrowedUtf8::new(mime_types)),
            Self::WindowSetClientFrame { window_id, frame } => window_set_client_side_decoration_frame(app_ptr, window_id, frame),
        }
    }
}
