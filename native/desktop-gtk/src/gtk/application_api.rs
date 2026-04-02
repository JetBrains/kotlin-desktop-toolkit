use crate::gtk::application::{Application, with_app_state, with_app_state_mut};
use crate::gtk::application_state::{EglInstance, get_egl, get_gl};
use crate::gtk::data_transfer_api::{DataSource, DragAndDropAction, DragAndDropActions, DragAndDropQueryData};
use crate::gtk::events::{EventHandler, RequestId, WindowId};
use crate::gtk::mime_types::MimeTypes;
use desktop_common::ffi_utils::RustAllocatedStrPtr;
use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedOpaquePtr, BorrowedStrPtr},
    logger::ffi_boundary,
};
use log::{debug, warn};

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderingMode {
    Auto,
    Gl,
    GlEs,
}

#[repr(C)]
#[derive(Debug)]
pub struct FfiSupportedActionsForMime<'a> {
    pub supported_mime_type: BorrowedStrPtr<'a>,
    pub supported_actions: DragAndDropActions,
    pub preferred_action: DragAndDropAction,
}

#[repr(C)]
#[derive(Debug)]
pub struct FfiDragAndDropQueryResponse {
    pub obj_id: i64,
    pub supported_actions_per_mime: BorrowedArray<'static, FfiSupportedActionsForMime<'static>>,
}

#[repr(C)]
#[derive(Debug)]
pub struct FfiTransferDataResponse {
    pub obj_id: i64,
    pub data: BorrowedArray<'static, u8>,
}

#[repr(C)]
#[derive(Debug)]
pub struct FfiTextInputSurroundingText {
    pub obj_id: i64,
    pub surrounding_text: BorrowedArray<'static, u8>,
    pub cursor_codepoint_offset: u16,
    pub selection_start_codepoint_offset: u16,
}

pub type FfiObjDealloc = extern "C" fn(i64);
pub type FfiWindowCloseRequest = extern "C" fn(WindowId) -> bool;
pub type FfiApplicationWantsToTerminate = extern "C" fn() -> bool;
pub type FfiQueryDragAndDropTarget = extern "C" fn(&DragAndDropQueryData) -> FfiDragAndDropQueryResponse;
pub type FfiTransferDataGetter = extern "C" fn(DataSource, BorrowedStrPtr) -> FfiTransferDataResponse;
pub type FfiRetrieveSurroundingText = extern "C" fn(WindowId) -> FfiTextInputSurroundingText;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ApplicationCallbacks {
    pub obj_dealloc: FfiObjDealloc,
    pub event_handler: EventHandler,
    pub query_drag_and_drop_target: FfiQueryDragAndDropTarget,
    pub get_data_transfer_data: FfiTransferDataGetter,
    pub window_close_request: FfiWindowCloseRequest,
    pub application_wants_to_terminate: FfiApplicationWantsToTerminate,
    pub retrieve_surrounding_text: FfiRetrieveSurroundingText,
}

#[unsafe(no_mangle)]
pub extern "C" fn application_init(app_id: BorrowedStrPtr) {
    ffi_boundary("application_init", || {
        debug!("Application Init");
        let app_id = app_id.as_str()?;
        Application::init(app_id)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_event_loop(callbacks: ApplicationCallbacks) {
    debug!("application_run_event_loop");
    ffi_boundary("application_run_event_loop", || Application::run_event_loop(&callbacks));
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_event_loop() {
    debug!("application_stop_event_loop");
    ffi_boundary("application_stop_event_loop", || {
        Application::stop_event_loop();
        Ok(())
    });
}

#[derive(Debug)]
#[repr(C)]
pub struct GetGlProcFuncData<'a> {
    pub f: extern "C" fn(ctx: BorrowedOpaquePtr<'a>, name: BorrowedStrPtr) -> *mut std::os::raw::c_void,
    pub ctx: BorrowedOpaquePtr<'a>,
}

/// cbindgen:ignore
extern "C" fn egl_get_proc_address(ctx_ptr: BorrowedOpaquePtr<'_>, name_ptr: BorrowedStrPtr) -> *mut std::os::raw::c_void {
    let name = name_ptr.as_str().unwrap();
    // debug!("egl_get_gl_proc for {name}");
    let egl = unsafe { ctx_ptr.borrow::<EglInstance>() }.expect("egl_get_proc_address: EGL Library not loaded");
    if let Some(f) = egl.get_proc_address(name) {
        f as *mut std::os::raw::c_void
    } else {
        std::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_egl_proc_func() -> GetGlProcFuncData<'static> {
    debug!("application_get_egl_proc_func");
    GetGlProcFuncData {
        f: egl_get_proc_address,
        ctx: BorrowedOpaquePtr::new(get_egl()),
    }
}

#[must_use]
pub fn gl_get_proc_address_impl(gl: &libloading::Library, name: &str) -> *mut std::os::raw::c_void {
    // debug!("egl_get_gl_proc for {name}");
    match unsafe { gl.get::<unsafe extern "C" fn()>(name.as_bytes()) } {
        Ok(sym) => {
            if let Some(p) = unsafe { sym.try_as_raw_ptr() } {
                p
            } else {
                warn!("Error getting {name}");
                std::ptr::null_mut()
            }
        }
        Err(e) => {
            warn!("Error getting {name}: {e}");
            std::ptr::null_mut()
        }
    }
}

/// cbindgen:ignore
extern "C" fn gl_get_proc_address(ctx_ptr: BorrowedOpaquePtr<'_>, name_ptr: BorrowedStrPtr) -> *mut std::os::raw::c_void {
    let name = name_ptr.as_str().unwrap();
    let gl = unsafe { ctx_ptr.borrow::<libloading::Library>() }.expect("gl_get_proc_address: GL Library not loaded");
    gl_get_proc_address_impl(gl, name)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_init_gl(gl_lib_path: BorrowedStrPtr) -> GetGlProcFuncData {
    let gl_lib_path = gl_lib_path.as_str().unwrap();
    if let Some(gl) = get_gl(gl_lib_path) {
        GetGlProcFuncData {
            f: gl_get_proc_address,
            ctx: BorrowedOpaquePtr::new(Some(gl)),
        }
    } else {
        GetGlProcFuncData {
            f: gl_get_proc_address,
            ctx: BorrowedOpaquePtr::null(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn application_is_event_loop_thread() -> bool {
    ffi_boundary("application_is_event_loop_thread", Application::is_event_loop_thread)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_on_event_loop_async(f: extern "C" fn()) {
    ffi_boundary("application_run_on_event_loop_async", || {
        #[allow(clippy::redundant_closure)]
        Application::run_on_event_loop_async(move || f());
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_put(mime_types: BorrowedStrPtr) {
    debug!("application_clipboard_put");
    ffi_boundary("application_clipboard_put", || {
        let mime_types = MimeTypes::new(mime_types.as_str()?);
        with_app_state_mut(|app| app.clipboard_put(&mime_types))
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_paste(serial: i32, supported_mime_types: BorrowedStrPtr) {
    let t = std::thread::current();
    debug!("application_clipboard_paste, thread id: {:?} ({:?})", t.id(), t.name());
    ffi_boundary("application_clipboard_paste", || {
        let supported_mime_types = MimeTypes::new(supported_mime_types.as_str()?);
        with_app_state(|app| {
            app.clipboard_paste(serial, &supported_mime_types);
            Ok(())
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_put(mime_types: BorrowedStrPtr) {
    debug!("application_primary_selection_put");
    ffi_boundary("application_primary_selection_put", || {
        let mime_types = MimeTypes::new(mime_types.as_str()?);
        with_app_state_mut(|app| app.primary_selection_put(&mime_types))
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_paste(serial: i32, supported_mime_types: BorrowedStrPtr) {
    let t = std::thread::current();
    debug!("application_clipboard_paste, thread id: {:?} ({:?})", t.id(), t.name());
    ffi_boundary("application_clipboard_paste", || {
        let supported_mime_types = MimeTypes::new(supported_mime_types.as_str()?);
        with_app_state(|app| {
            app.primary_selection_paste(serial, &supported_mime_types);
            Ok(())
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_get_available_mimetypes() -> RustAllocatedStrPtr {
    debug!("application_clipboard_get_available_mimetypes");
    ffi_boundary("application_clipboard_get_available_mimetypes", || {
        with_app_state(|app| {
            let csv_mimetypes = app.clipboard_get_available_mimetypes();
            Ok(RustAllocatedStrPtr::allocate(csv_mimetypes)?)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_get_available_mimetypes() -> RustAllocatedStrPtr {
    debug!("application_primary_selection_get_available_mimetypes");
    ffi_boundary("application_primary_selection_get_available_mimetypes", || {
        with_app_state(|app| {
            let csv_mimetypes = app.primary_selection_get_available_mimetypes();
            Ok(RustAllocatedStrPtr::allocate(csv_mimetypes)?)
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_request_redraw_drag_icon() {
    ffi_boundary("window_request_redraw", || {
        with_app_state(|app| {
            app.request_redraw_drag_icon();
            Ok(())
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_open_url(url_string: BorrowedStrPtr, activation_token: BorrowedStrPtr) -> RequestId {
    ffi_boundary("application_open_url", || {
        let url_string = url_string.as_str()?;
        let activation_token = activation_token.as_optional_str()?;
        with_app_state(|app| Ok(app.open_url(url_string, activation_token)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_open_file_manager(path: BorrowedStrPtr, activation_token: BorrowedStrPtr) -> RequestId {
    ffi_boundary("application_open_file_manager", || {
        let path = path.as_str()?.to_owned();
        let activation_token = activation_token.as_optional_str()?;
        with_app_state(|app| Ok(app.open_file_manager(path, activation_token)))
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
    title: BorrowedStrPtr,
    body: BorrowedStrPtr,
    sound_file_path: BorrowedStrPtr,
) -> RequestId {
    debug!("application_show_notification");
    ffi_boundary("application_show_notification", || {
        let summary = title.as_str()?.to_owned();
        let body = body.as_str()?.to_owned();
        let sound_file_path = sound_file_path.as_optional_str()?.map(ToOwned::to_owned);
        with_app_state_mut(|app| app.request_show_notification(summary, body, sound_file_path))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_close_notification(notification_id: u32) {
    debug!("application_show_notification");
    ffi_boundary("application_show_notification", || {
        with_app_state_mut(|app| app.request_close_notification(notification_id))
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_drag_and_drop() {
    ffi_boundary("application_stop_drag_and_drop", || {
        with_app_state(|app| {
            app.stop_drag();
            Ok(())
        })
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_set_prefer_dark_theme(value: bool) {
    debug!("application_set_prefer_dark_theme");
    ffi_boundary("application_set_prefer_dark_theme", || {
        with_app_state_mut(|app| {
            app.set_prefer_dark_theme(value);
            Ok(())
        })
    });
}
