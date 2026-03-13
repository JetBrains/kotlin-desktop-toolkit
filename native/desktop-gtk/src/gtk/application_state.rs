use crate::gtk::application::{Application, send_event, with_app_state_mut};
use crate::gtk::application_api::{ApplicationCallbacks, DataSource, FfiWindowCloseRequest, RenderingMode};
use crate::gtk::async_event_result::AsyncEventResult;
use crate::gtk::clipboard::{ClipboardContentProvider, KdtClipboard};
use crate::gtk::desktop_settings::DesktopSettings;
use crate::gtk::desktop_settings_api::FfiDesktopSetting;
use crate::gtk::events::Event::DragIconFrameTick;
use crate::gtk::events::{
    DataTransferCancelledEvent, DragAndDropFeedbackFinishedEvent, DragAndDropFinishedEvent, DragIconDrawEvent, Event, EventHandler,
    NotificationClosedEvent, OpenGlDrawData, RequestId, WindowClosedEvent, WindowId,
};
use crate::gtk::ffi_return_conversions::{QueryDragAndDropTarget, RetrieveSurroundingText, TransferDataGetter};
use crate::gtk::file_dialog::{show_open_file_dialog_impl, show_save_file_dialog_impl};
use crate::gtk::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use crate::gtk::geometry::{LogicalSize, PhysicalSize};
use crate::gtk::gl_widget::GlWidget;
use crate::gtk::kdt_application::KdtApplication;
use crate::gtk::mime_types::MimeTypes;
use crate::gtk::notifications::{close_notification_async, notifications_receiver, show_notification_async};
use crate::gtk::window::SimpleWindow;
use crate::gtk::window_api::WindowParams;
use anyhow::{Context, anyhow, bail};
use gtk4::glib::translate::ToGlibPtr;
use gtk4::prelude::{
    ActionMapExtManual, ApplicationExt, ApplicationExtManual, DisplayExt, DragExt, GtkApplicationExt, GtkWindowExt, ObjectExt, ObjectType,
    WidgetExt, WidgetExtManual,
};
use gtk4::{gdk as gdk4, gio, glib};
use log::{debug, warn};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CString, OsStr};
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::atomic::AtomicU32;
use std::sync::{LazyLock, OnceLock, atomic};
use tokio::spawn;

/// cbindgen:ignore
pub type EglInstance = khronos_egl::DynamicInstance<khronos_egl::EGL1_0>;

/// cbindgen:ignore
static EGL: LazyLock<Option<EglInstance>> = LazyLock::new(|| match unsafe { EglInstance::load() } {
    Ok(egl) => Some(egl),
    Err(e) => {
        warn!("Failed to load EGL: {e}");
        None
    }
});

/// cbindgen:ignore
pub static GL_INSTANCE: OnceLock<Option<libloading::Library>> = OnceLock::new();

pub fn get_gl(lib_path: impl AsRef<OsStr>) -> Option<&'static libloading::Library> {
    GL_INSTANCE
        .get_or_init(|| match unsafe { libloading::Library::new(lib_path.as_ref()) } {
            Ok(lib) => Some(lib),
            Err(e) => {
                warn!("Failed to load GL: {e}");
                None
            }
        })
        .as_ref()
}

pub fn get_egl() -> Option<&'static EglInstance> {
    match &*EGL {
        Some(v) => Some(v),
        None => None,
    }
}

#[cfg(target_os = "linux")]
#[allow(clippy::future_not_send)]
async fn get_ashpd_window_identifier(window: Option<&gtk4::Native>) -> Option<ashpd::WindowIdentifier> {
    if let Some(window) = window {
        ashpd::WindowIdentifier::from_native(window).await
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
#[allow(clippy::future_not_send, clippy::unused_async)]
async fn get_ashpd_window_identifier(_window: Option<&gtk4::Native>) -> Option<ashpd::WindowIdentifier> {
    None
}

pub struct ApplicationState {
    event_handler: EventHandler,
    window_close_request: FfiWindowCloseRequest,
    transfer_data_getter: TransferDataGetter,
    query_drag_and_drop_target: QueryDragAndDropTarget,
    retrieve_surrounding_text: RetrieveSurroundingText,
    _gtk_app_hold: gio::ApplicationHoldGuard,
    pub gtk_app: KdtApplication,
    rt: Rc<tokio::runtime::Runtime>,
    async_request_counter: AtomicU32,
    window_id_to_window: Rc<RefCell<HashMap<WindowId, SimpleWindow>>>,
    clipboard: KdtClipboard,
    primary_clipboard: KdtClipboard,
    current_drag: glib::WeakRef<gdk4::Drag>,
    drag_icon: Rc<RefCell<Option<GlWidget>>>,
    drag_content_provider: Rc<RefCell<Option<ClipboardContentProvider>>>,
    desktop_settings: DesktopSettings,
    notifications_connection_task: Option<tokio::task::JoinHandle<Option<zbus::Connection>>>,
    notifications_connection: Option<zbus::Connection>,
}

impl Drop for ApplicationState {
    fn drop(&mut self) {
        for window in self.gtk_app.windows() {
            self.gtk_app.remove_window(&window);
            window.destroy();
        }
        self.gtk_app.quit();
    }
}

impl ApplicationState {
    #[allow(clippy::too_many_lines)]
    pub fn new(callbacks: &ApplicationCallbacks) -> anyhow::Result<Self> {
        debug!("Initializing application state");

        let rt = tokio::runtime::Builder::new_multi_thread().enable_io().worker_threads(1).build()?;

        let gtk_app = KdtApplication::new();

        let application_wants_to_terminate = callbacks.application_wants_to_terminate;
        let event_handler = callbacks.event_handler;

        gtk_app.connect_activate(move |_gtk_app| {
            debug!("App Activate");
        });
        gtk_app.connect_startup(move |gtk_app| {
            debug!("App Startup");
            let quit = gio::ActionEntry::builder("quit")
                .activate(move |gtk_app: &KdtApplication, _, _| {
                    if application_wants_to_terminate() {
                        gtk_app.quit();
                    }
                })
                .build();
            gtk_app.add_action_entries([quit]);

            let initial_settings = with_app_state_mut(Self::read_and_subscribe_to_desktop_settings).unwrap();
            for initial_setting in initial_settings {
                send_event(event_handler, Event::DesktopSettingChange(initial_setting));
            }

            send_event(event_handler, Event::ApplicationStarted);

            debug!("After ApplicationStarted");
        });
        gtk_app.connect_shutdown(move |_gtk_app| {
            debug!("App Shutdown");
        });

        #[cfg(unix)]
        {
            let gtk_app = gtk_app.downgrade();
            // Handle SIGTERM
            glib_unix::unix_signal_add_local(15, move || {
                if application_wants_to_terminate()
                    && let Some(gtk_app) = gtk_app.upgrade()
                {
                    gtk_app.quit();
                }
                glib::ControlFlow::Continue
            });
        }

        let event_handler = callbacks.event_handler;
        let ffi_dealloc = callbacks.obj_dealloc;
        let display = gdk4::DisplayManager::get().default_display().context("Unable to open display")?;

        // If GSK decides to use Vulkan renderer, the OpenGL texture needs to be passed via DMA-BUF,
        // which is available only since GTK 4.14, and is an additional overhead.
        // Instead, force GTK to use OpenGL.
        // https://github.com/GNOME/gtk/blob/4.22.1/gsk/gskrenderer.c#L545
        unsafe {
            glib::gobject_ffi::g_object_set_qdata(
                display.as_object_ref().to_glib_none().0,
                glib::ffi::g_quark_from_string(c"gsk-renderer".as_ptr()),
                NonNull::from(c"opengl").as_ptr().cast(),
            );
        };

        let transfer_data_getter = TransferDataGetter {
            ffi_get: callbacks.get_data_transfer_data,
            ffi_dealloc,
        };
        let clipboard = KdtClipboard::new(display.clipboard(), DataSource::Clipboard, event_handler, transfer_data_getter);
        let primary_clipboard = KdtClipboard::new(
            display.primary_clipboard(),
            DataSource::PrimarySelection,
            event_handler,
            transfer_data_getter,
        );
        let desktop_settings = DesktopSettings::new(display);

        debug!("Trying to init notifications");
        let f = async move {
            match zbus::Connection::session().await {
                Ok(c) => {
                    let ret = c.clone();
                    spawn(notifications_receiver(c, move |notification_data| {
                        Application::run_on_event_loop_async(move || {
                            let action_cstring = notification_data.action.map(|v| CString::new(v).unwrap());
                            let activation_token_cstring = notification_data.activation_token.map(|v| CString::new(v).unwrap());
                            let e = NotificationClosedEvent::new(
                                notification_data.id,
                                action_cstring.as_ref(),
                                activation_token_cstring.as_ref(),
                            );
                            send_event(event_handler, e);
                        });
                        Ok(())
                    }));
                    Some(ret)
                }
                Err(e) => {
                    warn!("Error initializing notifications: {e}");
                    None
                }
            }
        };
        let notifications_connection_task = Some(rt.spawn(f));

        Ok(Self {
            event_handler,
            window_close_request: callbacks.window_close_request,
            transfer_data_getter,
            query_drag_and_drop_target: QueryDragAndDropTarget {
                ffi_get: callbacks.query_drag_and_drop_target,
                ffi_dealloc,
            },
            retrieve_surrounding_text: RetrieveSurroundingText {
                ffi_get: callbacks.retrieve_surrounding_text,
                ffi_dealloc,
            },
            _gtk_app_hold: gtk_app.hold(),
            gtk_app,
            rt: Rc::new(rt),
            async_request_counter: AtomicU32::new(1),
            window_id_to_window: Rc::new(RefCell::new(HashMap::new())),
            clipboard,
            primary_clipboard,
            current_drag: glib::WeakRef::default(),
            drag_icon: Rc::default(),
            drag_content_provider: Rc::default(),
            desktop_settings,
            notifications_connection_task,
            notifications_connection: None,
        })
    }

    /// Executes the future produced by the provided function.
    /// Return value is the same as the one passed to the function, representing the request id,
    /// so that the response (optionally produced by the future) can be matched to the request.
    pub fn run_async<F>(&self, f: impl FnOnce(RequestId) -> F) -> RequestId
    where
        F: Future<Output = AsyncEventResult> + Send + 'static,
        F::Output: Send + 'static,
    {
        let raw_request_id = self.async_request_counter.fetch_add(1, atomic::Ordering::Relaxed);
        let request_id = RequestId(raw_request_id);
        let future = f(request_id);
        let event_handler = self.event_handler;

        self.rt.spawn(async move {
            let val = future.await;
            glib::source::idle_add_once(move || {
                val.send_as_event(event_handler);
            });
        });
        request_id
    }

    pub fn run_async_for_window<F>(
        &self,
        window_id: WindowId,
        f: impl FnOnce(RequestId, Option<ashpd::WindowIdentifier>) -> F + 'static,
    ) -> anyhow::Result<RequestId>
    where
        F: Future<Output = AsyncEventResult> + Send + 'static,
        F::Output: Send + 'static,
    {
        let raw_request_id = self.async_request_counter.fetch_add(1, atomic::Ordering::Relaxed);
        let request_id = RequestId(raw_request_id);

        let rt = self.rt.clone();
        let event_handler = self.event_handler;

        let native = self.with_window(window_id, |w| Ok(w.window.upgrade().and_then(|w| w.native())))?;
        glib::spawn_future_local(async move {
            let identifier = get_ashpd_window_identifier(native.as_ref()).await;

            let future = f(request_id, identifier);

            rt.spawn(async move {
                let val = future.await;
                glib::source::idle_add_once(move || {
                    val.send_as_event(event_handler);
                });
            });
        });
        Ok(request_id)
    }

    pub fn read_and_subscribe_to_desktop_settings(&mut self) -> anyhow::Result<Vec<FfiDesktopSetting>> {
        let event_handler = self.event_handler;
        self.desktop_settings.read_and_subscribe(move |setting| {
            send_event(event_handler, Event::DesktopSettingChange(setting));
        })
    }

    fn get_or_wait_for_notifications_connection(&mut self) -> anyhow::Result<Option<zbus::Connection>> {
        if let Some(conn_handle) = self.notifications_connection_task.take() {
            self.notifications_connection = self.rt.block_on(conn_handle)?;
            Ok(self.notifications_connection.clone())
        } else if self.notifications_connection.is_none() {
            bail!("Didn't try initializing notifications");
        } else {
            Ok(self.notifications_connection.clone())
        }
    }

    pub fn new_window(&self, params: &WindowParams) -> anyhow::Result<()> {
        let window_id = params.window_id;
        let min_size = if params.min_size.height == 0 { None } else { Some(params.min_size) };

        if self.window_id_to_window.borrow().contains_key(&window_id) {
            bail!("Window with {window_id:?} already exists");
        }
        let event_handler = self.event_handler;
        let window_close_request = {
            let callback = self.window_close_request;
            move || {
                debug!("window_close_request {window_id:?} start");
                let close = callback(window_id);
                debug!("window_close_request {window_id:?} end ({close})");
                close
            }
        };
        let on_destroy = {
            let window_id_to_window_weak = Rc::downgrade(&self.window_id_to_window);
            move || {
                if let Some(window_id_to_window) = window_id_to_window_weak.upgrade() {
                    window_id_to_window.borrow_mut().remove(&window_id);
                }
                send_event(event_handler, WindowClosedEvent { window_id });
            }
        };
        let simple_window = SimpleWindow::new(
            &self.gtk_app,
            window_id,
            params.size,
            params.rendering_mode,
            min_size,
            params.title.as_optional_str()?,
            params.decoration_mode,
            event_handler,
            self.query_drag_and_drop_target,
            self.retrieve_surrounding_text,
            window_close_request,
            on_destroy,
        );
        self.window_id_to_window.borrow_mut().insert(window_id, simple_window);
        debug!("Created window {window_id:?}");

        // if let Some(screen_change_event) = screen_change_event {
        //     self.send_event(screen_change_event);
        // }

        // let xdg_titlebar_event =
        //     XdgDesktopSettingChange(XdgDesktopSetting::TitlebarLayout(BorrowedStrPtr::new(c":minimize,maximize,close")));
        // self.send_event(xdg_titlebar_event);
        Ok(())
    }

    pub fn close_window(&self, window_id: WindowId) {
        if let Some(w) = self.window_id_to_window.borrow().get(&window_id) {
            w.destroy();
        }
    }

    pub fn with_window<T>(&self, window_id: WindowId, f: impl FnOnce(&SimpleWindow) -> anyhow::Result<T>) -> anyhow::Result<T> {
        let borrowed = self.window_id_to_window.borrow();
        let window = borrowed
            .get(&window_id)
            .with_context(|| format!("Couldn't find window for {window_id:?}"))?;
        f(window)
    }

    pub fn clipboard_put(&mut self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        debug!("clipboard_put: mime_types={:?}", mime_types.val);
        self.clipboard.put(mime_types)
    }

    pub fn clipboard_get_available_mimetypes(&self) -> String {
        self.clipboard.mime_types_csv()
    }

    pub fn primary_selection_get_available_mimetypes(&self) -> String {
        self.primary_clipboard.mime_types_csv()
    }

    pub fn primary_selection_put(&mut self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        self.primary_clipboard.put(mime_types)
    }

    pub fn primary_selection_paste(&self, serial: i32, supported_mime_types: &MimeTypes) {
        self.primary_clipboard.paste(serial, supported_mime_types);
    }

    pub fn clipboard_paste(&self, serial: i32, supported_mime_types: &MimeTypes) {
        self.clipboard.paste(serial, supported_mime_types);
    }

    pub fn start_drag(
        &self,
        window_id: WindowId,
        mime_types: &MimeTypes,
        action: gdk4::DragAction,
        drag_icon_rendering_mode: RenderingMode,
        drag_icon_size: LogicalSize,
    ) -> anyhow::Result<()> {
        let provider = ClipboardContentProvider::new(mime_types, self.transfer_data_getter, DataSource::DragAndDrop);
        let drag = self.with_window(window_id, |w| w.start_drag_operation(action, &provider))?;
        let event_handler = self.event_handler;
        {
            let drag_icon = self.drag_icon.clone();
            let drag_content_provider = self.drag_content_provider.clone();
            drag.add_weak_ref_notify_local(move || {
                debug!("drag object destroyed");
                *drag_icon.borrow_mut() = None;
                *drag_content_provider.borrow_mut() = None;
                send_event(event_handler, DragAndDropFeedbackFinishedEvent { window_id });
            });
        }
        drag.connect_dnd_finished(move |drag| {
            let action = drag.selected_action();
            send_event(
                event_handler,
                DragAndDropFinishedEvent {
                    window_id,
                    action: action.into(),
                },
            );
        });
        drag.connect_cancel(move |_drag, reason| {
            warn!("Drag&drop cancelled: {reason:?}");
            send_event(
                event_handler,
                DataTransferCancelledEvent {
                    data_source: DataSource::DragAndDrop,
                },
            );
        });
        *self.drag_icon.borrow_mut() = if drag_icon_size.width > 0 && drag_icon_size.height > 0 {
            let on_draw = move |opengl_draw_data: OpenGlDrawData, physical_size: PhysicalSize, scale: f64| {
                send_event(
                    event_handler,
                    DragIconDrawEvent {
                        opengl_draw_data,
                        physical_size,
                        scale,
                    },
                );
            };
            let gl_widget = GlWidget::new(drag_icon_rendering_mode, Some(drag_icon_size), on_draw, None);

            gl_widget.add_tick_callback(move |_gl_widget, _frame_clock| {
                send_event(event_handler, DragIconFrameTick);
                glib::ControlFlow::Continue
            });
            let icon = gtk4::DragIcon::for_drag(&drag);
            icon.set_child(Some(&gl_widget));
            Some(gl_widget)
        } else {
            None
        };
        self.current_drag.set(Some(&drag));
        *self.drag_content_provider.borrow_mut() = Some(provider);

        Ok(())
    }

    pub fn stop_drag(&self) {
        if let Some(drag) = self.current_drag.upgrade() {
            drag.emit_by_name::<()>("cancel", &[&gdk4::DragCancelReason::UserCancelled]);
        }
    }

    pub fn request_redraw_drag_icon(&self) {
        (*self.drag_icon.borrow()).as_ref().map(GlWidget::queue_draw);
    }

    pub fn open_url(&self, url_string: &str, activation_token: Option<&str>) -> RequestId {
        debug!("application_open_url: {url_string}, activation_token={activation_token:?}");
        let raw_request_id = self.async_request_counter.fetch_add(1, atomic::Ordering::Relaxed);
        let request_id = RequestId(raw_request_id);
        gtk4::show_uri_full(
            gtk4::Window::NONE,
            url_string,
            gdk4::CURRENT_TIME,
            gio::Cancellable::NONE,
            move |res| {
                if let Err(e) = res {
                    warn!("Error trying to open URL for {request_id:?}: {e}");
                }
            },
        );

        request_id
    }

    async fn open_file_manager_impl(path: &str, request: ashpd::desktop::open_uri::OpenDirectoryRequest) -> anyhow::Result<()> {
        let f = tokio::fs::File::open(path).await?;
        request.send(&f).await?;
        Ok(())
    }

    pub fn open_file_manager(&self, path: String, activation_token: Option<&str>) -> RequestId {
        debug!("application_open_file_manager: {path}, activation_token={activation_token:?}");
        let request = ashpd::desktop::open_uri::OpenDirectoryRequest::default().activation_token(activation_token.map(Into::into));
        self.run_async(|request_id| async move {
            let error = Self::open_file_manager_impl(&path, request).await.err();
            AsyncEventResult::UrlOpenResponse { request_id, error }
        })
    }

    pub fn show_open_file_dialog(
        &self,
        window_id: WindowId,
        common_params: &CommonFileDialogParams,
        open_params: &OpenFileDialogParams,
    ) -> anyhow::Result<RequestId> {
        let request = common_params.create_open_request(open_params)?;

        self.run_async_for_window(window_id, |request_id, identifier| async move {
            let result = show_open_file_dialog_impl(identifier, request).await;
            AsyncEventResult::FileChooserResponse { request_id, result }
        })
    }

    pub fn show_save_file_dialog(
        &self,
        window_id: WindowId,
        common_params: &CommonFileDialogParams,
        save_params: &SaveFileDialogParams,
    ) -> anyhow::Result<RequestId> {
        let request = common_params.create_save_request(save_params)?;
        self.run_async_for_window(window_id, |request_id, identifier| async move {
            let result = show_save_file_dialog_impl(identifier, request).await;
            AsyncEventResult::FileChooserResponse { request_id, result }
        })
    }

    pub fn request_show_notification(
        &mut self,
        summary: String,
        body: String,
        sound_file_path: Option<String>,
    ) -> anyhow::Result<RequestId> {
        if let Some(conn) = self.get_or_wait_for_notifications_connection()? {
            Ok(self.run_async(|request_id| async move {
                let result = show_notification_async(&conn, &summary, &body, sound_file_path).await;
                AsyncEventResult::NotificationShown { request_id, result }
            }))
        } else {
            Err(anyhow!("Cannot interact with notifications"))
        }
    }

    pub fn request_close_notification(&mut self, notification_id: u32) -> anyhow::Result<()> {
        if let Some(conn) = self.get_or_wait_for_notifications_connection()? {
            self.run_async(move |_request_id| async move {
                if let Err(e) = close_notification_async(&conn, notification_id).await {
                    warn!("Error closing notification {notification_id}: {e}");
                }
                AsyncEventResult::NotificationClosed {}
            });
            Ok(())
        } else {
            Err(anyhow!("Cannot interact with notifications"))
        }
    }

    pub fn set_prefer_dark_theme(&self, value: bool) {
        self.desktop_settings.set_prefer_dark_theme(value);
    }
}
