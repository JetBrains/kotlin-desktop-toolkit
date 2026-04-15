use desktop_common::ffi_utils::BorrowedUtf8;
use desktop_gtk::gtk::text_input_api::TextInputContext;
use desktop_gtk::gtk::{
    application_api::{
        RenderingMode,
        application_clipboard_paste,
        application_clipboard_put,
        application_primary_selection_paste,
        application_primary_selection_put,
        application_set_prefer_dark_theme,
        application_stop_drag_and_drop,
        application_stop_event_loop,
        //
    },
    events::{WindowDecorationMode, WindowId},
    geometry::LogicalSize,
    window_api::{
        WindowParams,
        window_activate,
        window_close,
        window_create,
        window_maximize,
        window_set_fullscreen,
        window_text_input_disable,
        window_text_input_enable,
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
    ApplicationSetPreferDarkTheme(bool),
    ApplicationStopEventLoop,
    ApplicationStopDragAndDrop,
    WindowActivate {
        window_id: WindowId,
        token: Option<String>,
    },
    WindowCreate {
        window_id: WindowId,
        size: LogicalSize,
        min_size: LogicalSize,
        title: String,
        decoration_mode: WindowDecorationMode,
        rendering_mode: RenderingMode,
    },
    WindowClose(WindowId),
    WindowMaximize(WindowId),
    WindowSetFullscreen(WindowId),
    WindowUnmaximize(WindowId),
    WindowUnsetFullscreen(WindowId),
    WindowTextInputDisable(WindowId),
    WindowTextInputEnable(WindowId, TextInputContext),
}

impl Action {
    pub fn perform(self) {
        match self {
            Self::Dummy => {}
            Self::ApplicationSetPreferDarkTheme(v) => application_set_prefer_dark_theme(v),
            Self::ApplicationStopDragAndDrop => application_stop_drag_and_drop(),
            Self::ApplicationStopEventLoop => application_stop_event_loop(),
            Self::WindowActivate { window_id, token } => window_activate(window_id, BorrowedUtf8::optional(token.as_ref())),
            Self::WindowCreate {
                window_id,
                size,
                min_size,
                title,
                decoration_mode,
                rendering_mode,
            } => {
                window_create(WindowParams {
                    window_id,
                    size,
                    min_size,
                    title: BorrowedUtf8::new(&title),
                    decoration_mode,
                    rendering_mode,
                });
            }
            Self::WindowClose(window_id) => window_close(window_id),
            Self::WindowMaximize(window_id) => window_maximize(window_id),
            Self::WindowSetFullscreen(window_id) => window_set_fullscreen(window_id),
            Self::WindowUnmaximize(window_id) => window_unmaximize(window_id),
            Self::WindowUnsetFullscreen(window_id) => window_unset_fullscreen(window_id),
            Self::WindowTextInputDisable(window_id) => window_text_input_disable(window_id),
            Self::WindowTextInputEnable(window_id, context) => window_text_input_enable(window_id, context),
            Self::ApplicationClipboardPaste {
                serial,
                supported_mime_types,
            } => application_clipboard_paste(serial, BorrowedUtf8::new(supported_mime_types)),
            Self::ApplicationClipboardPut(mime_types) => application_clipboard_put(BorrowedUtf8::new(mime_types)),
            Self::ApplicationPrimarySelectionPaste {
                serial,
                supported_mime_types,
            } => application_primary_selection_paste(serial, BorrowedUtf8::new(supported_mime_types)),
            Self::ApplicationPrimarySelectionPut(mime_types) => application_primary_selection_put(BorrowedUtf8::new(mime_types)),
        }
    }
}
