use anyhow::{Context, bail};
use desktop_common::{
    ffi_utils::{BorrowedArray, BorrowedOpaquePtr, BorrowedStrPtr, RustAllocatedRawPtr, RustAllocatedStrPtr},
    logger::ffi_boundary,
};
use log::debug;
use smithay_client_toolkit::reexports::client::protocol::wl_data_device_manager::DndAction;

use crate::linux::{
    application::Application,
    application_state::EglInstance,
    data_transfer::MimeTypes,
    events::{EventHandler, WindowId},
    geometry::LogicalPoint,
    text_input_api::TextInputContext,
    xdg_desktop_settings_api::XdgDesktopSetting,
};

#[repr(C)]
#[derive(Debug)]
pub struct DragAndDropQueryData {
    pub window_id: WindowId,
    pub point: LogicalPoint,
}

#[repr(C)]
#[derive(Debug)]
pub enum DataSource {
    Clipboard,
    DragAndDrop,
}

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    pub on_application_started: extern "C" fn(),
    // Returns true if application should terminate, otherwise termination will be canceled
    pub on_should_terminate: extern "C" fn() -> bool,
    pub on_will_terminate: extern "C" fn(),
    pub on_display_configuration_change: extern "C" fn(),
    pub on_xdg_desktop_settings_change: extern "C" fn(&XdgDesktopSetting),
    pub event_handler: EventHandler,
    pub get_drag_and_drop_supported_mime_types: extern "C" fn(&DragAndDropQueryData) -> BorrowedStrPtr<'static>,
    pub get_data_transfer_data: extern "C" fn(DataSource, BorrowedStrPtr) -> BorrowedArray<'static, u8>,
    pub on_data_transfer_cancelled: extern "C" fn(DataSource),
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

extern "C" fn egl_get_proc_address(ctx_ptr: BorrowedOpaquePtr<'_>, name_ptr: BorrowedStrPtr) -> Option<extern "system" fn()> {
    let name = name_ptr.as_str().unwrap();
    // debug!("egl_get_gl_proc for {name}");
    let egl = unsafe { ctx_ptr.borrow::<EglInstance>() }.expect("egl_get_proc_address: EGL Library not loaded");
    egl.get_proc_address(name)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_egl_proc_func(app_ptr: AppPtr<'_>) -> GetEglProcFuncData<'_> {
    debug!("application_get_egl_proc_func");
    let app = unsafe { app_ptr.borrow::<Application>() };
    GetEglProcFuncData {
        f: egl_get_proc_address,
        ctx: BorrowedOpaquePtr::new(app.state.egl.as_ref()),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn application_is_event_loop_thread(app_ptr: AppPtr<'_>) -> bool {
    ffi_boundary("application_is_event_loop_thread", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(t) = app.state.event_loop_thread_id {
            let current_thread_id = std::thread::current().id();
            Ok(t == current_thread_id)
        } else {
            bail!("Event loop not yet started")
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn application_run_on_event_loop_async(app_ptr: AppPtr<'_>, f: extern "C" fn()) {
    ffi_boundary("application_run_on_event_loop_async", || {
        let app = unsafe { app_ptr.borrow::<Application>() };
        if let Some(s) = &app.state.run_on_event_loop {
            s.send(f).map_err(std::convert::Into::into)
        } else {
            bail!("Event loop not yet started")
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_set_cursor_theme(mut app_ptr: AppPtr<'_>, name: BorrowedStrPtr, size: u32) {
    debug!("application_set_cursor_theme");
    ffi_boundary("application_set_cursor_theme", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.set_cursor_theme(name.as_str().unwrap(), size)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_enable(mut app_ptr: AppPtr<'_>, context: TextInputContext) {
    debug!("application_text_input_enable {context:?}");
    ffi_boundary("application_text_input_enable", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let text_input = app.state.active_text_input.as_mut().context("Active text input")?;
        text_input.enable();
        context.apply(text_input)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_update(mut app_ptr: AppPtr<'_>, context: TextInputContext) {
    debug!("application_text_input_update {context:?}");
    ffi_boundary("application_text_input_update", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        let text_input = app.state.active_text_input.as_mut().context("Active text input")?;
        context.apply(text_input)
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_text_input_disable(mut app_ptr: AppPtr<'_>) {
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
pub extern "C" fn application_clipboard_put(mut app_ptr: AppPtr<'_>, mime_types: BorrowedStrPtr) {
    debug!("application_clipboard_put");
    ffi_boundary("application_clipboard_put", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.clipboard_put(MimeTypes::new(mime_types.as_str()?));
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_clipboard_get_available_mimetypes(mut app_ptr: AppPtr<'_>) -> RustAllocatedStrPtr {
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
pub extern "C" fn application_start_drag_and_drop(
    mut app_ptr: AppPtr,
    window_id: WindowId,
    mime_types: BorrowedStrPtr,
    action: DragAction,
) {
    debug!("application_start_drag_and_drop");
    ffi_boundary("application_start_drag_and_drop", || {
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.start_drag(window_id, MimeTypes::new(mime_types.as_str()?), action.into())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_open_url(url_string: BorrowedStrPtr) -> bool {
    debug!("application_open_url");
    ffi_boundary("application_open_url", || {
        let uri = ashpd::url::Url::parse(url_string.as_str()?)?;
        async_std::task::block_on(ashpd::desktop::open_uri::OpenFileRequest::default().ask(false).send_uri(&uri))?;
        Ok(true)
    })
}
