use crate::gtk::application_api::{ApplicationCallbacks, DataSource};
use crate::gtk::application_state::ApplicationState;
use crate::gtk::async_event_result::AsyncEventResult;
use crate::gtk::drag_icon::DragIcon;
use crate::gtk::events::{
    DataTransferCancelledEvent, DragAndDropFinishedEvent, Event, EventHandler, NotificationClosedEvent, RequestId, WindowId,
};
use crate::gtk::file_dialog::{show_open_file_dialog_impl, show_save_file_dialog_impl};
use crate::gtk::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use crate::gtk::geometry::LogicalSize;
use crate::gtk::kdt_application::KdtApplication;
use crate::gtk::mime_types::MimeTypes;
use crate::gtk::notifications::{close_notification_async, notifications_receiver, show_notification_async};
use crate::gtk::window::SimpleWindow;
use crate::gtk::window_api::WindowParams;
use anyhow::{Context, anyhow, bail};
use desktop_common::logger::catch_panic;
use gtk4::prelude::{
    ActionMapExtManual, ApplicationExt, ApplicationExtManual, DragExt, GtkApplicationExt, GtkWindowExt, ObjectExt, WidgetExt,
};
use gtk4::{gdk as gdk4, gio, glib};
use log::{debug, warn};
use std::ffi::CString;
use std::rc::Rc;
use std::sync::atomic;
use std::sync::atomic::AtomicU32;
use std::thread::ThreadId;
use tokio::spawn;

pub struct Application {
    gtk_app: Option<KdtApplication>,
    gtk_app_hold: Option<gio::ApplicationHoldGuard>,
    pub state: ApplicationState,
    pub event_loop_thread_id: Option<ThreadId>,
    rt: Rc<tokio::runtime::Runtime>,
    async_request_counter: AtomicU32,
}

#[allow(clippy::needless_pass_by_value)]
pub fn send_event<'a, T: Into<Event<'a>>>(event_handler: EventHandler, event_data: T) {
    let event: Event = event_data.into();
    match event {
        Event::MouseMoved(_) | Event::ShouldRedraw(_) | Event::WindowDraw(_) | Event::DragIconDraw(_) => {}
        _ => debug!("Sending event: {event:?}"),
    }
    catch_panic(|| {
        event_handler(&event);
        Ok(())
    })
    .unwrap_or(());
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

impl Application {
    pub fn new(callbacks: ApplicationCallbacks) -> anyhow::Result<Self> {
        let state = ApplicationState::new(callbacks);

        let rt = Rc::new(tokio::runtime::Builder::new_multi_thread().enable_io().worker_threads(1).build()?);

        Ok(Self {
            gtk_app: None,
            gtk_app_hold: None,
            state,
            event_loop_thread_id: None,
            rt,
            async_request_counter: AtomicU32::new(1),
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
        let event_handler = self.state.callbacks.event_handler;

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
        let event_handler = self.state.callbacks.event_handler;

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

    pub fn run(&mut self, app_id: &str) -> Result<(), anyhow::Error> {
        debug!("Application event loop: starting");

        gtk4::init()?;
        let app = KdtApplication::new(app_id);
        self.gtk_app = Some(app.clone());
        self.event_loop_thread_id = Some(std::thread::current().id());
        self.state.init()?;
        self.init_notifications();

        let application_wants_to_terminate = self.state.callbacks.application_wants_to_terminate;
        let event_handler = self.state.callbacks.event_handler;

        app.connect_activate(move |_app| {
            debug!("App Activate");
        });
        app.connect_startup(move |app| {
            debug!("App Startup");
            let quit = gio::ActionEntry::builder("quit")
                .activate(move |app: &KdtApplication, _, _| {
                    if application_wants_to_terminate() {
                        app.quit();
                    }
                })
                .build();
            app.add_action_entries([quit]);

            send_event(event_handler, Event::ApplicationStarted);
            debug!("After ApplicationStarted");
        });
        app.connect_shutdown(move |_app| {
            debug!("App Shutdown");
        });

        #[cfg(unix)]
        {
            let app = app.downgrade();
            // Handle SIGTERM
            glib::source::unix_signal_add_local(15, move || {
                if application_wants_to_terminate()
                    && let Some(app) = app.upgrade()
                {
                    app.quit();
                }
                glib::ControlFlow::Continue
            });
        }

        self.gtk_app_hold = Some(app.hold());
        debug!("Before App run");
        app.run();

        debug!("Application event loop: stopped");
        Ok(())
    }

    pub fn stop_event_loop(&mut self) {
        if let Some(gtk_app) = self.gtk_app.take() {
            for window in gtk_app.windows() {
                gtk_app.remove_window(&window);
                window.destroy();
            }
            gtk_app.quit();
        }
        self.gtk_app = None;
        self.gtk_app_hold = None;
    }

    pub fn run_on_event_loop_async<F>(func: F) -> glib::SourceId
    where
        F: FnOnce() + Send + 'static,
    {
        glib::source::idle_add_once(func)
    }

    pub fn new_window(&self, params: &WindowParams) -> anyhow::Result<()> {
        let window_id = params.window_id;
        let min_size = if params.min_size.height == 0 { None } else { Some(params.min_size) };

        self.state.create_window(
            self.gtk_app.as_ref().context("Application not initialized")?,
            window_id,
            params.size,
            min_size,
            params.title.as_optional_str()?,
            params.prefer_client_side_decoration,
        );
        Ok(())
    }

    pub fn close_window(&self, window_id: WindowId) {
        if let Some(w) = self.state.window_id_to_window.borrow_mut().get(&window_id) {
            w.destroy();
        }
    }

    pub fn with_window<T>(&self, window_id: WindowId, f: impl FnOnce(&SimpleWindow) -> anyhow::Result<T>) -> anyhow::Result<T> {
        let borrowed = self.state.window_id_to_window.borrow();
        let window = borrowed
            .get(&window_id)
            .with_context(|| format!("Couldn't find window for {window_id:?}"))?;
        f(window)
    }

    pub fn clipboard_put(&mut self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        debug!("clipboard_put: mime_types={:?}", mime_types.val);
        if let Some(clipboard) = self.state.clipboard.as_mut() {
            clipboard.put(mime_types)
        } else {
            bail!("Clipboard not initialized");
        }
    }

    pub fn clipboard_get_available_mimetypes(&self) -> Option<String> {
        let clipboard = self.state.clipboard.as_ref()?;
        Some(clipboard.mime_types_csv())
    }

    pub fn primary_selection_get_available_mimetypes(&self) -> Option<String> {
        let clipboard = self.state.primary_clipboard.as_ref()?;
        Some(clipboard.mime_types_csv())
    }

    pub fn primary_selection_put(&mut self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        if let Some(clipboard) = self.state.primary_clipboard.as_mut() {
            clipboard.put(mime_types)
        } else {
            bail!("Primary selection not initialized");
        }
    }

    pub fn primary_selection_paste(&self, serial: i32, supported_mime_types: &MimeTypes) -> bool {
        if let Some(primary_clipboard) = &self.state.primary_clipboard {
            primary_clipboard.paste(serial, supported_mime_types);
            true
        } else {
            false
        }
    }

    pub fn clipboard_paste(&self, serial: i32, supported_mime_types: &MimeTypes) -> bool {
        if let Some(clipboard) = &self.state.clipboard {
            clipboard.paste(serial, supported_mime_types);
            true
        } else {
            false
        }
    }

    pub fn start_drag(
        &mut self,
        window_id: WindowId,
        mime_types: &MimeTypes,
        action: gdk4::DragAction,
        drag_icon_size: LogicalSize,
    ) -> anyhow::Result<()> {
        let drag = self.with_window(window_id, |w| {
            w.start_drag_operation(mime_types, action, self.state.callbacks.get_data_transfer_data)
        })?;
        let event_handler = self.state.callbacks.event_handler;
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
        self.state.drag_icon = if drag_icon_size.width > 0 && drag_icon_size.height > 0 {
            Some(DragIcon::new(event_handler, &drag, drag_icon_size)?)
        } else {
            None
        };
        self.state.current_drag.set(Some(&drag));

        Ok(())
    }

    pub fn stop_drag(&self) {
        if let Some(drag) = self.state.current_drag.upgrade() {
            drag.drop_done(false);
        }
    }

    pub fn request_redraw_drag_icon(&self) {
        self.state.drag_icon.as_ref().map(DragIcon::request_redraw);
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

    pub fn init_notifications(&self) {
        debug!("Trying to init notifications");
        let notifications_connection = self.state.notifications_connection.clone();
        let event_handler = self.state.callbacks.event_handler;
        self.rt.spawn(async move {
            match zbus::Connection::session().await {
                Ok(c) => {
                    {
                        let c = c.clone();
                        Self::run_on_event_loop_async(move || {
                            *notifications_connection.write().unwrap() = Some(c);
                        });
                    }
                    spawn(notifications_receiver(c, move |notification_data| {
                        Self::run_on_event_loop_async(move || {
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
                }
                Err(e) => warn!("Error initializing notifications: {e}"),
            }
        });
    }

    pub fn request_show_notification(&self, summary: String, body: String, sound_file_path: Option<String>) -> anyhow::Result<RequestId> {
        let conn = self.state.notifications_connection.clone();
        let conn = conn.read().map_err(|e| anyhow!(e.to_string()))?;
        if let Some(conn) = conn.clone() {
            Ok(self.run_async(|request_id| async move {
                let result = show_notification_async(&conn, &summary, &body, sound_file_path).await;
                AsyncEventResult::NotificationShown { request_id, result }
            }))
        } else {
            Err(anyhow!("Cannot interact with notifications"))
        }
    }

    pub fn request_close_notification(&self, notification_id: u32) -> anyhow::Result<()> {
        let conn = self.state.notifications_connection.clone();
        let conn = conn.read().map_err(|e| anyhow!(e.to_string()))?;
        if let Some(conn) = conn.clone() {
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
}
