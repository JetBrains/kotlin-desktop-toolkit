use crate::linux::application_api::{ApplicationCallbacks, DataSource};
use crate::linux::application_state::ApplicationState;
use crate::linux::async_event_result::AsyncEventResult;
use crate::linux::clipboard::{ClipboardContentProvider, clipboard_paste_impl};
use crate::linux::drag_icon::DragIcon;
use crate::linux::events::{
    DataTransferCancelledEvent, DragAndDropFinishedEvent, Event, EventHandler, NotificationClosedEvent, RequestId, WindowId,
};
use crate::linux::file_dialog::{show_open_file_dialog_impl, show_save_file_dialog_impl};
use crate::linux::file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams};
use crate::linux::geometry::LogicalSize;
use crate::linux::mime_types::MimeTypes;
use crate::linux::notifications::{close_notification_async, notifications_receiver, show_notification_async};
use crate::linux::window::SimpleWindow;
use crate::linux::window_api::WindowParams;
use crate::linux::window_resize_edge_api::WindowResizeEdge;
use anyhow::{Context, anyhow, bail};
use desktop_common::logger::catch_panic;
use gtk4::prelude::{DragExt, GtkWindowExt, WidgetExt};
use gtk4::{gdk as gdk4, glib};
use log::{debug, warn};
use std::ffi::CString;
use std::rc::Rc;
use std::sync::atomic;
use std::sync::atomic::AtomicU32;
use std::thread::ThreadId;
use tokio::spawn;

pub struct Application {
    exit: bool,
    main_context: glib::MainContext,
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

impl Application {
    pub fn new(callbacks: ApplicationCallbacks) -> anyhow::Result<Self> {
        let main_context = glib::MainContext::default();
        let state = ApplicationState::new(callbacks);

        let rt = Rc::new(tokio::runtime::Builder::new_multi_thread().enable_io().worker_threads(1).build()?);

        Ok(Self {
            exit: false,
            main_context,
            state,
            event_loop_thread_id: None,
            rt,
            async_request_counter: AtomicU32::new(0),
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

        let native = self.with_window(window_id, |w| Ok(w.window.native()))?;
        glib::spawn_future_local(async move {
            let identifier = if let Some(window) = native {
                ashpd::WindowIdentifier::from_native(&window).await
            } else {
                None
            };

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

        glib::set_prgname(Some(app_id));
        gtk4::init()?;
        self.event_loop_thread_id = Some(std::thread::current().id());
        self.state.init()?;
        self.init_notifications();

        self.state.send_event(Event::ApplicationStarted);

        debug!("After ApplicationStarted");

        #[allow(clippy::while_immutable_condition)]
        while !self.exit {
            self.main_context.iteration(true);
            // debug!("Event loop iteration");

            for window_id in self.state.windows_to_close.take() {
                // First, call `destroy` without removing from `window_id_to_window`, because destroying can trigger events,
                // which then can call another `window_` function, and we want this window ID to still be valid.
                let gtk_window = if let Some(w) = self.state.window_id_to_window.borrow().get(&window_id) {
                    // Don't hold a borrow during `destroy`, because destroying can trigger events,
                    // which then can call another `window_` function, which would then cause `BorrowError`.
                    w.window.clone()
                } else {
                    continue;
                };
                gtk_window.destroy();

                self.state.window_id_to_window.borrow_mut().remove(&window_id);
            }
        }

        debug!("Application event loop: stopped");
        Ok(())
    }

    pub fn stop_event_loop(&mut self) {
        self.exit = true;
        self.main_context.wakeup();
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
            window_id,
            params.size,
            min_size,
            params.title.as_optional_str()?,
            params.prefer_client_side_decoration,
        )
    }

    pub fn close_window(&self, window_id: WindowId) {
        // Close the window asynchronously, because closing can trigger events, which then can cause reentrancy issues in the client code.
        self.state.windows_to_close.borrow_mut().push(window_id);
        self.main_context.wakeup();
    }

    pub fn with_window<T>(&self, window_id: WindowId, f: impl FnOnce(&SimpleWindow) -> anyhow::Result<T>) -> anyhow::Result<T> {
        let borrowed = self.state.window_id_to_window.borrow();
        let window = borrowed
            .get(&window_id)
            .with_context(|| format!("Couldn't find window for {window_id:?}"))?;
        f(window)
    }

    // pub fn set_cursor_theme(&mut self, name: &str, size: u32) -> anyhow::Result<()> {
    //     self.state.set_cursor_theme(&self.qh, name, size)
    // }

    fn clipboard_put_impl(&self, mime_types: &MimeTypes, clipboard: &gdk4::Clipboard) -> anyhow::Result<()> {
        let get_data_transfer_data = self.state.callbacks.get_data_transfer_data;
        let provider = ClipboardContentProvider::new(mime_types, get_data_transfer_data);
        clipboard.set_content(Some(&provider))?;
        Ok(())
    }

    pub fn clipboard_put(&self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        debug!("clipboard_put: mime_types={:?}", mime_types.val);
        if let Some(clipboard) = self.state.clipboard.as_ref() {
            self.clipboard_put_impl(mime_types, clipboard)
        } else {
            bail!("Clipboard not initialized");
        }
    }

    pub fn clipboard_get_available_mimetypes(&self) -> Option<String> {
        let clipboard = self.state.clipboard.as_ref()?;
        Some(clipboard.formats().mime_types().join(","))
    }

    pub fn primary_selection_get_available_mimetypes(&self) -> Option<String> {
        let clipboard = self.state.primary_clipboard.as_ref()?;
        Some(clipboard.formats().mime_types().join(","))
    }

    pub fn primary_selection_put(&self, mime_types: &MimeTypes) -> anyhow::Result<()> {
        if let Some(clipboard) = self.state.primary_clipboard.as_ref() {
            self.clipboard_put_impl(mime_types, clipboard)
        } else {
            bail!("Primary selection not initialized");
        }
    }

    pub fn primary_selection_paste(&self, serial: i32, supported_mime_types: &MimeTypes) -> bool {
        if let Some(primary_clipboard) = &self.state.primary_clipboard {
            clipboard_paste_impl(self.state.callbacks.event_handler, serial, supported_mime_types, primary_clipboard);
            true
        } else {
            false
        }
    }

    pub fn clipboard_paste(&self, serial: i32, supported_mime_types: &MimeTypes) -> bool {
        if let Some(clipboard) = &self.state.clipboard {
            clipboard_paste_impl(self.state.callbacks.event_handler, serial, supported_mime_types, clipboard);
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
            let action = drag.actions();
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

    pub fn window_activate(&self, window_id: WindowId, token: Option<&str>) -> anyhow::Result<()> {
        debug!("window_activate: {window_id:?}, token={token:?}");
        self.with_window(window_id, |w| {
            w.focus(token);
            Ok(())
        })
    }

    pub fn window_start_move(&self, window_id: WindowId) -> anyhow::Result<()> {
        self.with_window(window_id, SimpleWindow::start_move)
    }

    pub fn window_start_resize(&self, window_id: WindowId, edge: WindowResizeEdge) -> anyhow::Result<()> {
        self.with_window(window_id, |w| w.start_resize(edge))
    }

    pub fn window_show_menu(&self, window_id: WindowId) -> anyhow::Result<()> {
        self.with_window(window_id, SimpleWindow::show_menu)
    }

    pub fn open_url(&self, url_string: &str, activation_token: Option<&str>) -> anyhow::Result<RequestId> {
        debug!("application_open_url: {url_string}, activation_token={activation_token:?}");
        let uri = ashpd::url::Url::parse(url_string)?;
        let request = ashpd::desktop::open_uri::OpenFileRequest::default()
            .ask(false)
            .activation_token(activation_token.map(Into::into));

        Ok(self.run_async(|request_id| async move {
            let error = request.send_uri(&uri).await.err().map(Into::into);
            AsyncEventResult::UrlOpenResponse { request_id, error }
        }))
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
                            let activation_token_cstring = notification_data.activation_token.map(|v| CString::new(v).unwrap());
                            let e = NotificationClosedEvent::new(notification_data.id, activation_token_cstring.as_ref());
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
