use anyhow::{Context, bail};
use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedOpaquePtr, BorrowedStrPtr, RustAllocatedRawPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use log::debug;

use crate::linux::{
    application::Application,
    application_state::{EglInstance, get_egl},
    data_transfer::MimeTypes,
    events::{EventHandler, RequestId, WindowId},
    geometry::LogicalPoint,
    text_input_api::TextInputContext,
};

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DragAndDropAction {
    None = 0b0000_0000,

    Copy = 0b0000_0001,

    Move = 0b0000_0010,
    // TODO?: Ask
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct DragAndDropActions(pub u8);

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderingMode {
    Auto,
    Software,
    EGL,
}

#[repr(C)]
#[derive(Debug)]
pub struct DragAndDropQueryData {
    pub window_id: WindowId,
    pub location_in_window: LogicalPoint,
}

#[repr(C)]
#[derive(Debug)]
pub struct SupportedActionsForMime<'a> {
    pub supported_mime_type: BorrowedStrPtr<'a>,
    pub supported_actions: DragAndDropActions,
    pub preferred_action: DragAndDropAction,
}

#[repr(C)]
#[derive(Debug)]
pub struct DragAndDropQueryResponse<'a> {
    pub supported_actions_per_mime: BorrowedArray<'a, SupportedActionsForMime<'a>>,
}

#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub enum DataSource {
    Clipboard,
    DragAndDrop,
    PrimarySelection,
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    pub event_handler: EventHandler,
    pub query_drag_and_drop_target: extern "C" fn(&DragAndDropQueryData) -> DragAndDropQueryResponse,
    pub get_data_transfer_data: extern "C" fn(DataSource, BorrowedStrPtr) -> BorrowedArray<'_, u8>,
}

pub type AppPtr<'a> = RustAllocatedRawPtr<'a>;

#[unsafe(no_mangle)]
pub extern "C" fn application_init(callbacks: ApplicationCallbacks) -> AppPtr<'static> {
    let app = ffi_boundary("application_init", || {
        debug!("Application Init");
        Ok(Some(Application::new(callbacks)?))
    });
    AppPtr::from_value(app)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_event_loop(mut app_ptr: AppPtr) {
    ffi_boundary("application_run_event_loop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.run()
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_event_loop(mut app_ptr: AppPtr) {
    debug!("application_stop_event_loop");
    ffi_boundary("application_stop_event_loop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.exit = true;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_shutdown(app_ptr: AppPtr) {
    debug!("application_shutdown");
    ffi_boundary("application_shutdown", || {
        let _app = unsafe { app_ptr.to_owned::<Application>() };
        Ok(())
    });
}

#[derive(Debug)]
#[repr(C)]
pub struct GetEglProcFuncData<'a> {
    pub f: extern "C" fn(ctx: BorrowedOpaquePtr<'a>, name: BorrowedStrPtr) -> Option<extern "system" fn()>,
    pub ctx: BorrowedOpaquePtr<'a>,
}

/// cbindgen:ignore
extern "C" fn egl_get_proc_address(ctx_ptr: BorrowedOpaquePtr<'_>, name_ptr: BorrowedStrPtr) -> Option<extern "system" fn()> {
    let name = name_ptr.as_str().unwrap();
    // debug!("egl_get_gl_proc for {name}");
    let egl = unsafe { ctx_ptr.borrow::<EglInstance>() }.expect("egl_get_proc_address: EGL Library not loaded");
    egl.get_proc_address(name)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_egl_proc_func(_app_ptr: AppPtr<'_>) -> GetEglProcFuncData<'_> {
    debug!("application_get_egl_proc_func");
    GetEglProcFuncData {
        f: egl_get_proc_address,
        ctx: BorrowedOpaquePtr::new(get_egl()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn application_is_event_loop_thread(app_ptr: AppPtr) -> bool {
    ffi_boundary("application_is_event_loop_thread", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(t) = app.event_loop_thread_id {
            let current_thread_id = std::thread::current().id();
            Ok(t == current_thread_id)
        } else {
            bail!("Event loop not yet started")
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_on_event_loop_async(app_ptr: AppPtr, f: extern "C" fn()) {
    ffi_boundary("application_run_on_event_loop_async", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(s) = &app.run_on_event_loop {
            s.send(f).map_err(std::convert::Into::into)
        } else {
            bail!("Event loop not yet started")
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_set_cursor_theme(mut app_ptr: AppPtr, name: BorrowedStrPtr, size: u32) {
    debug!("application_set_cursor_theme");
    ffi_boundary("application_set_cursor_theme", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.set_cursor_theme(name.as_str().unwrap(), size)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_enable(mut app_ptr: AppPtr, context: TextInputContext) {
    debug!("application_text_input_enable {context:?}");
    ffi_boundary("application_text_input_enable", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let text_input = app.state.active_text_input.as_mut().context("Active text input")?;
        text_input.enable();
        context.apply(text_input)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_update(mut app_ptr: AppPtr, context: TextInputContext) {
    debug!("application_text_input_update {context:?}");
    ffi_boundary("application_text_input_update", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let text_input = app.state.active_text_input.as_mut().context("Active text input")?;
        context.apply(text_input)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_disable(mut app_ptr: AppPtr) {
    debug!("application_text_input_disable");
    ffi_boundary("application_text_input_disable", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let text_input = app.state.active_text_input.as_mut().context("Active text input")?;
        text_input.disable();
        text_input.commit();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_put(mut app_ptr: AppPtr, mime_types: BorrowedStrPtr) {
    debug!("application_clipboard_put");
    ffi_boundary("application_clipboard_put", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.clipboard_put(MimeTypes::new(mime_types.as_str()?));
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_paste(app_ptr: AppPtr<'_>, serial: i32, supported_mime_types: BorrowedStrPtr) -> bool {
    let t = std::thread::current();
    debug!("application_clipboard_paste, thread id: {:?} ({:?})", t.id(), t.name());
    ffi_boundary("application_clipboard_paste", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        Ok(app.clipboard_paste(serial, supported_mime_types.as_str()?))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_put(mut app_ptr: AppPtr, mime_types: BorrowedStrPtr) {
    debug!("application_primary_selection_put");
    ffi_boundary("application_primary_selection_put", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.primary_selection_put(MimeTypes::new(mime_types.as_str()?));
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_paste(app_ptr: AppPtr<'_>, serial: i32, supported_mime_types: BorrowedStrPtr) -> bool {
    let t = std::thread::current();
    debug!("application_clipboard_paste, thread id: {:?} ({:?})", t.id(), t.name());
    ffi_boundary("application_clipboard_paste", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        Ok(app.primary_selection_paste(serial, supported_mime_types.as_str()?))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_get_available_mimetypes(mut app_ptr: AppPtr) -> RustAllocatedStrPtr {
    debug!("application_clipboard_get_available_mimetypes");
    ffi_boundary("application_clipboard_get_available_mimetypes", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        if let Some(csv_mimetypes) = app.clipboard_get_available_mimetypes() {
            Ok(RustAllocatedStrPtr::allocate(csv_mimetypes)?)
        } else {
            Ok(RustAllocatedStrPtr::null())
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_get_available_mimetypes(mut app_ptr: AppPtr) -> RustAllocatedStrPtr {
    debug!("application_primary_selection_get_available_mimetypes");
    ffi_boundary("application_primary_selection_get_available_mimetypes", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        if let Some(csv_mimetypes) = app.primary_selection_get_available_mimetypes() {
            Ok(RustAllocatedStrPtr::allocate(csv_mimetypes)?)
        } else {
            Ok(RustAllocatedStrPtr::null())
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_open_url(mut app_ptr: AppPtr, url_string: BorrowedStrPtr) {
    debug!("application_open_url");
    ffi_boundary("application_open_url", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.open_url(url_string.as_str()?)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_request_internal_activation_token(app_ptr: AppPtr, source_window_id: WindowId) -> u32 {
    ffi_boundary("application_request_activation_token", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.request_internal_activation_token(source_window_id)
    })
}

/// * `title`: User-visible string to display as the title.
///   This should be a short string, if it doesn’t fit the UI, it may be truncated to fit on a single line.
/// * `body`: User-visible string to display as the body.
///   This can be a long string, but if it doesn’t fit the UI, it may be wrapped or/and truncated.
/// * `sound_file_path`: The path to a sound file to play when the notification pops up.
///   The mandatory supported sound file formats are WAV/PCM 8-48kHz, 8/16bits, and OGG/Vorbis I.
#[unsafe(no_mangle)]
pub extern "C" fn application_request_show_notification(
    mut app_ptr: AppPtr,
    title: BorrowedStrPtr,
    body: BorrowedStrPtr,
    sound_file_path: BorrowedStrPtr,
) -> RequestId {
    debug!("application_show_notification");
    ffi_boundary("application_show_notification", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let summary = title.as_str()?.to_owned();
        let body = body.as_str()?.to_owned();
        let sound_file_path = sound_file_path.as_optional_str()?.map(ToOwned::to_owned);
        app.request_show_notification(summary, body, sound_file_path)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_close_notification(mut app_ptr: AppPtr, notification_id: u32) {
    debug!("application_show_notification");
    ffi_boundary("application_show_notification", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.request_close_notification(notification_id)
    });
}
