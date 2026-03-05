use crate::gtk::application::Application;
use crate::gtk::application_state::{EglInstance, get_egl, get_gl};
use crate::gtk::events::{EventHandler, RequestId, WindowId};
use crate::gtk::geometry::LogicalPoint;
use crate::gtk::mime_types::MimeTypes;
use anyhow::bail;
use desktop_common::ffi_utils::RustAllocatedStrPtr;
use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedOpaquePtr, BorrowedStrPtr, RustAllocatedRawPtr},
    logger::ffi_boundary,
};
use log::{debug, warn};

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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataSource {
    Clipboard,
    DragAndDrop,
    PrimarySelection,
}

pub type QueryDragAndDropTarget = extern "C" fn(&DragAndDropQueryData) -> DragAndDropQueryResponse;
pub type FfiTransferDataGetter = extern "C" fn(DataSource, BorrowedStrPtr) -> BorrowedArray<'_, u8>;
pub type FfiWindowCloseRequest = extern "C" fn(WindowId) -> bool;
pub type FfiApplicationWantsToTerminate = extern "C" fn() -> bool;

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    pub event_handler: EventHandler,
    pub query_drag_and_drop_target: QueryDragAndDropTarget,
    pub get_data_transfer_data: FfiTransferDataGetter,
    pub window_close_request: FfiWindowCloseRequest,
    pub application_wants_to_terminate: FfiApplicationWantsToTerminate,
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
pub extern "C" fn application_run_event_loop(mut app_ptr: AppPtr, app_id: BorrowedStrPtr) {
    let thread_id = std::thread::current().id();
    debug!("application_run_event_loop; thread_id={thread_id:?}");
    ffi_boundary("application_run_event_loop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let app_id = app_id.as_str()?;
        app.run(app_id)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_event_loop(mut app_ptr: AppPtr) {
    debug!("application_stop_event_loop");
    ffi_boundary("application_stop_event_loop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.stop_event_loop();
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
pub extern "C" fn application_run_on_event_loop_async(_app_ptr: AppPtr, f: extern "C" fn()) {
    ffi_boundary("application_run_on_event_loop_async", || {
        #[allow(clippy::redundant_closure)]
        Application::run_on_event_loop_async(move || f());
        Ok(())
    });
}

// #[unsafe(no_mangle)]
// pub extern "C" fn application_set_cursor_theme(mut app_ptr: AppPtr, name: BorrowedStrPtr, size: u32) {
//     debug!("application_set_cursor_theme");
//     ffi_boundary("application_set_cursor_theme", || {
//         let app = unsafe { app_ptr.borrow_mut::<Application>() };
//         app.set_cursor_theme(name.as_str().unwrap(), size)
//     });
// }

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_put(mut app_ptr: AppPtr, mime_types: BorrowedStrPtr) {
    debug!("application_clipboard_put");
    ffi_boundary("application_clipboard_put", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let mime_types = MimeTypes::new(mime_types.as_str()?);
        app.clipboard_put(&mime_types)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_paste(app_ptr: AppPtr<'_>, serial: i32, supported_mime_types: BorrowedStrPtr) -> bool {
    let t = std::thread::current();
    debug!("application_clipboard_paste, thread id: {:?} ({:?})", t.id(), t.name());
    ffi_boundary("application_clipboard_paste", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let supported_mime_types = MimeTypes::new(supported_mime_types.as_str()?);
        Ok(app.clipboard_paste(serial, &supported_mime_types))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_put(mut app_ptr: AppPtr, mime_types: BorrowedStrPtr) {
    debug!("application_primary_selection_put");
    ffi_boundary("application_primary_selection_put", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let mime_types = MimeTypes::new(mime_types.as_str()?);
        app.primary_selection_put(&mime_types)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_paste(app_ptr: AppPtr<'_>, serial: i32, supported_mime_types: BorrowedStrPtr) -> bool {
    let t = std::thread::current();
    debug!("application_clipboard_paste, thread id: {:?} ({:?})", t.id(), t.name());
    ffi_boundary("application_clipboard_paste", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let supported_mime_types = MimeTypes::new(supported_mime_types.as_str()?);
        Ok(app.primary_selection_paste(serial, &supported_mime_types))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_get_available_mimetypes(app_ptr: AppPtr) -> RustAllocatedStrPtr {
    debug!("application_clipboard_get_available_mimetypes");
    ffi_boundary("application_clipboard_get_available_mimetypes", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(csv_mimetypes) = app.clipboard_get_available_mimetypes() {
            Ok(RustAllocatedStrPtr::allocate(csv_mimetypes)?)
        } else {
            Ok(RustAllocatedStrPtr::null())
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_primary_selection_get_available_mimetypes(app_ptr: AppPtr) -> RustAllocatedStrPtr {
    debug!("application_primary_selection_get_available_mimetypes");
    ffi_boundary("application_primary_selection_get_available_mimetypes", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(csv_mimetypes) = app.primary_selection_get_available_mimetypes() {
            Ok(RustAllocatedStrPtr::allocate(csv_mimetypes)?)
        } else {
            Ok(RustAllocatedStrPtr::null())
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_request_redraw_drag_icon(app_ptr: AppPtr) {
    ffi_boundary("window_request_redraw", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.request_redraw_drag_icon();
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_open_url(app_ptr: AppPtr, url_string: BorrowedStrPtr, activation_token: BorrowedStrPtr) -> RequestId {
    ffi_boundary("application_open_url", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let request_id = app.open_url(url_string.as_str()?, activation_token.as_optional_str()?);
        Ok(request_id)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_open_file_manager(app_ptr: AppPtr, path: BorrowedStrPtr, activation_token: BorrowedStrPtr) -> RequestId {
    ffi_boundary("application_open_file_manager", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        Ok(app.open_file_manager(path.as_str()?.to_owned(), activation_token.as_optional_str().unwrap()))
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
    app_ptr: AppPtr,
    title: BorrowedStrPtr,
    body: BorrowedStrPtr,
    sound_file_path: BorrowedStrPtr,
) -> RequestId {
    debug!("application_show_notification");
    ffi_boundary("application_show_notification", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        let summary = title.as_str()?.to_owned();
        let body = body.as_str()?.to_owned();
        let sound_file_path = sound_file_path.as_optional_str()?.map(ToOwned::to_owned);
        app.request_show_notification(summary, body, sound_file_path)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_close_notification(app_ptr: AppPtr, notification_id: u32) {
    debug!("application_show_notification");
    ffi_boundary("application_show_notification", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.request_close_notification(notification_id)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_stop_drag_and_drop(app_ptr: AppPtr) {
    ffi_boundary("application_stop_drag_and_drop", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        app.stop_drag();
        Ok(())
    });
}
