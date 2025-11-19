use std::{ffi::CString, thread::ThreadId, time::Duration};

use anyhow::{Context, anyhow};
use log::{debug, warn};
use smithay_client_toolkit::{
    reexports::{
        calloop::{
            Dispatcher, EventLoop,
            channel::{self, Sender},
        },
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, Proxy as _, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_data_device_manager::DndAction, wl_surface::WlSurface},
        },
    },
    shell::WaylandSurface,
};
use tokio::spawn;

use crate::linux::{
    application_api::{ApplicationCallbacks, RenderingMode},
    application_state::{ApplicationState, get_egl},
    async_event_result::AsyncEventResult,
    data_transfer::{MimeTypes, read_from_pipe},
    drag_icon::DragIcon,
    events::{DataTransferEvent, Event, NotificationClosedEvent, RequestId, WindowId},
    file_dialog::{show_open_file_dialog_impl, show_save_file_dialog_impl},
    file_dialog_api::{CommonFileDialogParams, OpenFileDialogParams, SaveFileDialogParams},
    geometry::{LogicalPoint, LogicalSize},
    notifications::{NotificationData, close_notification_async, notifications_receiver, show_notification_async},
    window::SimpleWindow,
    window_api::WindowParams,
    window_resize_edge_api::WindowResizeEdge,
    xdg_desktop_settings::xdg_desktop_settings_notifier,
    xdg_desktop_settings_api::XdgDesktopSetting,
};

pub struct Application {
    pub event_loop: EventLoop<'static, ApplicationState>,
    qh: QueueHandle<ApplicationState>,
    pub exit: bool,
    pub state: ApplicationState,
    pub run_on_event_loop: Option<Sender<extern "C" fn()>>,
    pub event_loop_thread_id: Option<ThreadId>,
    rt: tokio::runtime::Runtime,
    async_request_counter: u32,
    run_async_sender: Sender<AsyncEventResult>,
}

fn create_run_async_sender(event_loop: &EventLoop<'static, ApplicationState>) -> Sender<AsyncEventResult> {
    let (sender, c) = channel::channel();

    event_loop
        .handle()
        .insert_source(c, move |event: channel::Event<AsyncEventResult>, (), state| {
            if let channel::Event::Msg(e) = event {
                e.send_as_event(state.callbacks.event_handler);
            }
        })
        .unwrap();

    sender
}

impl Application {
    pub fn new(callbacks: ApplicationCallbacks) -> anyhow::Result<Self> {
        let conn = Connection::connect_to_env()?;

        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh: QueueHandle<ApplicationState> = event_queue.handle();

        let event_loop = EventLoop::<ApplicationState>::try_new()?;
        let loop_handle = event_loop.handle();
        let wl_display = conn.display();

        WaylandSource::new(conn, event_queue)
            .insert(loop_handle)
            .map_err(|e| anyhow!(e.to_string()))?;

        let state = ApplicationState::new(&globals, &qh, callbacks, event_loop.handle(), wl_display);
        let rt = tokio::runtime::Builder::new_multi_thread().enable_io().worker_threads(1).build()?;
        let run_async_sender = create_run_async_sender(&event_loop);

        Ok(Self {
            event_loop,
            qh,
            exit: false,
            state,
            run_on_event_loop: None,
            event_loop_thread_id: None,
            rt,
            async_request_counter: 0,
            run_async_sender,
        })
    }

    /// Executes the future produced by the provided function.
    /// Return value is the same as the one passed to the function, representing the request id,
    /// so that the response (optionally produced by the future) can be matched to the request.
    pub fn run_async<F>(&mut self, f: impl FnOnce(RequestId) -> F) -> RequestId
    where
        F: Future<Output = AsyncEventResult> + Send + 'static,
        F::Output: Send + 'static,
    {
        self.async_request_counter = self.async_request_counter.wrapping_add(1);
        let request_id = RequestId(self.async_request_counter);
        let future = f(request_id);
        let sender = self.run_async_sender.clone();

        self.rt.spawn(async move {
            sender.send(future.await).unwrap();
        });
        request_id
    }

    fn init_xdg_desktop_settings_notifier(&self) {
        let (xdg_settings_sender, xdg_settings_channel) = channel::channel();
        self.rt.spawn(xdg_desktop_settings_notifier(move |s| {
            xdg_settings_sender.send(s).map_err(Into::into)
        }));

        self.event_loop
            .handle()
            .insert_source(xdg_settings_channel, move |event, (), state| {
                if let channel::Event::Msg(e) = event {
                    XdgDesktopSetting::with(e, |s| {
                        state.send_event(Event::XdgDesktopSettingChange(s));
                    });
                }
            })
            .unwrap();
    }

    fn init_run_on_event_loop(&mut self) {
        let (s, c) = channel::channel();
        self.run_on_event_loop = Some(s);

        self.event_loop
            .handle()
            .insert_source(c, move |event, (), _state| {
                if let channel::Event::Msg(e) = event {
                    e();
                }
            })
            .unwrap();
    }

    pub fn event_loop_iteration(&mut self) -> Result<bool, anyhow::Error> {
        self.event_loop.dispatch(Duration::from_millis(16), &mut self.state)?;

        for (k, v) in self.state.windows.extract_if(|_, v| v.close) {
            debug!("Closing window {:?} ({k})", v.window_id);
            self.state.window_id_to_surface_id.remove(&v.window_id);
        }

        if self.exit && !self.state.send_event(Event::ApplicationWantsToTerminate) {
            debug!("Exiting");
            self.state.send_event(Event::ApplicationWillTerminate);
            self.state.windows.clear();
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub fn run(&mut self) -> Result<(), anyhow::Error> {
        debug!("Application event loop: starting");

        self.init_notifications();
        self.init_xdg_desktop_settings_notifier();
        self.init_run_on_event_loop();

        self.event_loop_thread_id = Some(std::thread::current().id());
        self.state.send_event(Event::ApplicationStarted);

        while self.event_loop_iteration()? {
            // debug!("Application event loop: continuing");
        }
        debug!("Application event loop: stopped");
        Ok(())
    }

    pub fn new_window(&mut self, params: &WindowParams) -> anyhow::Result<()> {
        let window_id = params.window_id;
        if self.state.window_id_to_surface_id.contains_key(&window_id) {
            return Err(anyhow!("Window with ID {window_id:?} already exists"));
        }
        let w = SimpleWindow::new(window_id, &self.state, &self.qh, params);
        let surface_id = w.window.wl_surface().id();
        self.state.windows.insert(surface_id.clone(), w);
        self.state.window_id_to_surface_id.insert(window_id, surface_id);
        Ok(())
    }

    pub fn get_window(&self, window_id: WindowId) -> anyhow::Result<&SimpleWindow> {
        self.state
            .window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.state.windows.get(surface_id))
            .with_context(|| format!("Couldn't find window for {window_id:?}"))
    }

    pub fn get_window_mut(&mut self, window_id: WindowId) -> anyhow::Result<&mut SimpleWindow> {
        self.state
            .window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.state.windows.get_mut(surface_id))
            .with_context(|| format!("Couldn't find window for {window_id:?}"))
    }

    pub fn set_cursor_theme(&mut self, name: &str, size: u32) -> anyhow::Result<()> {
        self.state.set_cursor_theme(&self.qh, name, size)
    }

    pub fn clipboard_put(&mut self, mime_types: MimeTypes) {
        if mime_types.val.is_empty() {
            self.state.copy_paste_source = None;
            warn!("application_clipboard_put: Passed mime_types are empty");
            return;
        }
        let Some(device) = self.state.data_device.as_ref() else {
            warn!("application_clipboard_put: No data device");
            return;
        };
        if let Some((_seat, serial)) = self.state.get_latest_event_seat_and_serial() {
            let copy_paste_source = self
                .state
                .data_device_manager_state
                .create_copy_paste_source(&self.qh, mime_types.val);
            copy_paste_source.set_selection(device, serial);
            self.state.copy_paste_source = Some(copy_paste_source);
        } else {
            warn!("application_clipboard_put: No last event serial");
        }
    }

    pub fn clipboard_get_available_mimetypes(&self) -> Option<String> {
        let Some(data_device) = self.state.data_device.as_ref() else {
            warn!("application_clipboard_get_available_mimetypes: No data device available");
            return None;
        };
        let Some(selection_offer) = data_device.data().selection_offer() else {
            debug!("application_clipboard_get_available_mimetypes: No selection offer found");
            return None;
        };
        selection_offer.with_mime_types(|mime_types| Some(mime_types.join(",")))
    }

    pub fn primary_selection_get_available_mimetypes(&self) -> Option<String> {
        let Some(device) = self.state.primary_selection_device.as_ref() else {
            warn!("application_primary_selection_get_available_mimetypes: No primary selection device");
            return None;
        };
        let Some(selection_offer) = device.data().selection_offer() else {
            debug!("application_primary_selection_get_available_mimetypes: No selection offer found");
            return None;
        };
        selection_offer.with_mime_types(|mime_types| Some(mime_types.join(",")))
    }

    pub fn primary_selection_put(&mut self, mime_types: MimeTypes) {
        if mime_types.val.is_empty() {
            self.state.primary_selection_source = None;
            warn!("application_primary_selection_put: None");
            return;
        }
        let Some(device) = self.state.primary_selection_device.as_ref() else {
            warn!("application_primary_selection_put: No primary selection device");
            return;
        };
        let Some(manager) = self.state.primary_selection_manager.as_ref() else {
            warn!("application_primary_selection_put: No primary selection manager");
            return;
        };
        if let Some((_seat, serial)) = self.state.get_latest_event_seat_and_serial() {
            let source = manager.create_selection_source(&self.qh, mime_types.val);
            source.set_selection(device, serial);
            self.state.primary_selection_source = Some(source);
        } else {
            warn!("application_primary_selection_put: No last event serial");
        }
    }

    pub fn primary_selection_paste(&self, serial: i32, supported_mime_types: &str) -> bool {
        let Some(device) = self.state.primary_selection_device.as_ref() else {
            return false;
        };

        let Some(offer) = device.data().selection_offer() else {
            debug!("application_primary_selection_paste: No selection offer found");
            return false;
        };
        let Some(mime_type) = offer.with_mime_types(|mime_types| {
            debug!("application_primary_selection_paste: offer MIME types: {mime_types:?}, supported MIME types: {supported_mime_types}");
            supported_mime_types
                .split(',')
                .find(|&supported_mime_type| mime_types.iter().any(|m| m == supported_mime_type))
                .map(str::to_owned)
        }) else {
            debug!("application_primary_selection_paste: clipboard content not supported");
            return false;
        };
        debug!("application_primary_selection_paste: reading {mime_type}");
        let read_pipe = match offer.receive(mime_type.clone()) {
            Ok(v) => v,
            Err(e) => {
                warn!("application_primary_selection_paste: failed receive the data offer: {e}");
                return false;
            }
        };
        read_from_pipe(
            "application_primary_selection_paste",
            read_pipe,
            mime_type,
            &self.state.loop_handle,
            move |state, content| {
                state.send_event(DataTransferEvent { serial, content });
            },
        )
    }

    pub fn clipboard_paste(&self, serial: i32, supported_mime_types: &str) -> bool {
        let Some(device) = self.state.data_device.as_ref() else {
            warn!("application_clipboard_paste: No data device available");
            return false;
        };
        let Some(offer) = device.data().selection_offer() else {
            debug!("application_clipboard_paste: No selection offer found");
            return false;
        };
        let Some(mime_type) = offer.with_mime_types(|mime_types| {
            debug!("application_clipboard_paste: offer MIME types: {mime_types:?}, supported MIME types: {supported_mime_types}");
            supported_mime_types
                .split(',')
                .find(|&supported_mime_type| mime_types.iter().any(|m| m == supported_mime_type))
                .map(str::to_owned)
        }) else {
            debug!("application_clipboard_paste: clipboard content not supported");
            return false;
        };

        debug!("application_clipboard_paste: reading {mime_type}");
        let read_pipe = match offer.receive(mime_type.clone()) {
            Ok(v) => v,
            Err(e) => {
                warn!("application_clipboard_paste: failed receive the data offer: {e}");
                return false;
            }
        };
        read_from_pipe(
            "application_clipboard_paste",
            read_pipe,
            mime_type,
            &self.state.loop_handle,
            move |state, content| {
                state.send_event(DataTransferEvent { serial, content });
            },
        )
    }

    pub fn start_drag(
        &mut self,
        window_id: WindowId,
        mime_types: MimeTypes,
        action: DndAction,
        drag_icon_rendering_mode: RenderingMode,
        drag_icon_size: LogicalSize,
    ) -> anyhow::Result<()> {
        if mime_types.val.is_empty() {
            self.state.drag_source = None;
            return Ok(());
        }

        let mut drag_icon = if drag_icon_size.width > 0 && drag_icon_size.height > 0 {
            let egl = match drag_icon_rendering_mode {
                RenderingMode::Auto | RenderingMode::EGL => get_egl(),
                RenderingMode::Software => None,
            };

            Some(DragIcon::new(
                &self.state,
                &self.qh,
                &self.state.shm_state,
                &self.state.wl_display,
                drag_icon_size,
                egl,
            )?)
        } else {
            None
        };

        let origin = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?
            .window
            .wl_surface();

        let drag_source = self
            .state
            .data_device_manager_state
            .create_drag_and_drop_source(&self.qh, mime_types.val, action);

        let device = self.state.data_device.as_ref().context("No data device found")?;

        // Required to have a mouse button pressed serial, e.g.:
        // https://gitlab.gnome.org/GNOME/mutter/-/blob/607a7aef5f02d3213b5e436d11440997478a4ecc/src/wayland/meta-wayland-data-device.c#L894
        let (_seat, serial) = self
            .state
            .get_latest_pointer_button_seat_and_serial()
            .context("Called start_drag without an implicit grab")?;

        let wl_surface = drag_icon.as_ref().map(|drag_icon| drag_icon.surface.wl_surface());
        drag_source.start_drag(device, origin, wl_surface, serial);
        if let Some(drag_icon) = &mut drag_icon {
            drag_icon.draw(&self.qh, &|_e| true);
        }

        self.state.drag_icon = drag_icon;
        self.state.current_drag_source_window_id = Some(window_id);
        self.state.drag_source = Some(drag_source);

        Ok(())
    }

    pub fn request_internal_activation_token(&self, source_window_id: WindowId) -> anyhow::Result<u32> {
        debug!("request_internal_activation_token: source_window_id={source_window_id:?}");
        let source_w: &SimpleWindow = self
            .get_window(source_window_id)
            .with_context(|| format!("No window found {source_window_id:?}"))?;
        let Some(xdg_activation) = &self.state.xdg_activation else {
            warn!("xdg_activation not found");
            return Ok(0);
        };
        // Serial should be from the latest keyboard focus or mouse button down event, e.g.:
        // https://gitlab.gnome.org/GNOME/mutter/-/blob/607a7aef5f02d3213b5e436d11440997478a4ecc/src/wayland/meta-wayland-activation.c#L302
        // https://gitlab.gnome.org/GNOME/mutter/-/blob/607a7aef5f02d3213b5e436d11440997478a4ecc/src/wayland/meta-wayland-pointer.c#L760
        // https://invent.kde.org/plasma/kwin/-/blob/271eae7f21326b48e67de1ed218caf3bdf96a9c5/src/activation.cpp#L640
        let Some((seat, serial)) = self.state.get_latest_event_seat_and_serial() else {
            return Ok(0);
        };
        let request_id = serial + 1; // aligned with `impl ActivationHandler for ApplicationState`
        xdg_activation.request_token(
            &self.qh,
            smithay_client_toolkit::activation::RequestData {
                app_id: Some(source_w.app_id.clone()),
                seat_and_serial: Some((seat.clone(), serial)),
                surface: Some(source_w.window.wl_surface().clone()),
            },
        );
        Ok(request_id)
    }

    pub fn window_activate(&self, window_id: WindowId, token: String) -> anyhow::Result<()> {
        debug!("window_activate: {window_id:?}, token={token}");
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        let Some(xdg_activation) = &self.state.xdg_activation else {
            warn!("xdg_activation not found");
            return Ok(());
        };
        xdg_activation.activate::<ApplicationState>(w.window.wl_surface(), token);
        Ok(())
    }

    pub fn window_start_move(&self, window_id: WindowId) -> anyhow::Result<()> {
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        // Required to have a mouse button pressed serial, e.g.:
        // https://gitlab.gnome.org/GNOME/mutter/-/blob/607a7aef5f02d3213b5e436d11440997478a4ecc/src/wayland/meta-wayland-xdg-shell.c#L335
        if let Some((seat, serial)) = self.state.get_latest_pointer_button_seat_and_serial() {
            w.start_move(seat, serial);
        }
        Ok(())
    }

    pub fn window_start_resize(&self, window_id: WindowId, edge: WindowResizeEdge) -> anyhow::Result<()> {
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        // Required to have a mouse button pressed serial, e.g.:
        // https://gitlab.gnome.org/GNOME/mutter/-/blob/607a7aef5f02d3213b5e436d11440997478a4ecc/src/wayland/meta-wayland-xdg-shell.c#L387
        if let Some((seat, serial)) = self.state.get_latest_pointer_button_seat_and_serial() {
            w.start_resize(edge, seat, serial);
        }
        Ok(())
    }

    pub fn window_show_menu(&self, window_id: WindowId, position: LogicalPoint) -> anyhow::Result<()> {
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        // Required to have a mouse button pressed serial, e.g.:
        // https://gitlab.gnome.org/GNOME/mutter/-/blob/607a7aef5f02d3213b5e436d11440997478a4ecc/src/wayland/meta-wayland-xdg-shell.c#L309
        if let Some((seat, serial)) = self.state.get_latest_pointer_button_seat_and_serial() {
            debug!("window_show_menu: seat: {seat:?}, serial: {serial:?}");
            w.show_menu(position, seat, serial);
        }
        Ok(())
    }

    pub fn open_url(&mut self, url_string: &str, activation_token: Option<&str>) -> anyhow::Result<RequestId> {
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

    pub fn open_file_manager(&mut self, path: String, activation_token: Option<&str>) -> RequestId {
        debug!("application_open_file_manager: {path}, activation_token={activation_token:?}");
        let request = ashpd::desktop::open_uri::OpenDirectoryRequest::default().activation_token(activation_token.map(Into::into));
        self.run_async(|request_id| async move {
            let error = Self::open_file_manager_impl(&path, request).await.err();
            AsyncEventResult::UrlOpenResponse { request_id, error }
        })
    }

    fn get_wl_surface(&self, window_id: WindowId) -> anyhow::Result<WlSurface> {
        Ok(self.get_window(window_id)?.window.wl_surface().clone())
    }

    pub fn show_open_file_dialog(
        &mut self,
        window_id: WindowId,
        common_params: &CommonFileDialogParams,
        open_params: &OpenFileDialogParams,
    ) -> anyhow::Result<RequestId> {
        let wl_surface = self.get_wl_surface(window_id)?;
        let request = common_params.create_open_request(open_params)?;
        Ok(self.run_async(|request_id| async move {
            let identifier = ashpd::WindowIdentifier::from_wayland(&wl_surface).await;
            let result = show_open_file_dialog_impl(identifier, request).await;
            AsyncEventResult::FileChooserResponse { request_id, result }
        }))
    }

    pub fn show_save_file_dialog(
        &mut self,
        window_id: WindowId,
        common_params: &CommonFileDialogParams,
        save_params: &SaveFileDialogParams,
    ) -> anyhow::Result<RequestId> {
        let wl_surface = self.get_wl_surface(window_id)?;
        let request = common_params.create_save_request(save_params)?;
        Ok(self.run_async(|request_id| async move {
            let identifier: Option<ashpd::WindowIdentifier> = ashpd::WindowIdentifier::from_wayland(&wl_surface).await;
            let result = show_save_file_dialog_impl(identifier, request).await;
            AsyncEventResult::FileChooserResponse { request_id, result }
        }))
    }

    pub fn init_notifications(&self) {
        let (sender, c) = channel::channel();

        let dispatcher = Dispatcher::new(
            c,
            move |event: channel::Event<zbus::Connection>, (), state: &mut ApplicationState| {
                if let channel::Event::Msg(c) = event {
                    state.notifications_connection = Some(c);
                }
            },
        );
        self.event_loop.handle().register_dispatcher(dispatcher).unwrap();

        let (event_sender, event_c) = channel::channel();
        self.event_loop
            .handle()
            .insert_source(event_c, move |event: channel::Event<NotificationData>, (), state| {
                if let channel::Event::Msg(notification_data) = event {
                    let activation_token_cstring = notification_data.activation_token.map(|v| CString::new(v).unwrap());
                    let e = NotificationClosedEvent::new(notification_data.id, activation_token_cstring.as_ref());
                    state.send_event(e);
                }
            })
            .unwrap();
        self.rt.spawn(async move {
            match zbus::Connection::session().await {
                Ok(c) => {
                    sender.send(c.clone()).unwrap();
                    spawn(notifications_receiver(c, move |s| event_sender.send(s).map_err(Into::into)));
                }
                Err(e) => warn!("Error initializing notifications: {e}"),
            }
        });
    }

    pub fn request_show_notification(
        &mut self,
        summary: String,
        body: String,
        sound_file_path: Option<String>,
    ) -> anyhow::Result<RequestId> {
        let Some(conn) = &self.state.notifications_connection else {
            return Err(anyhow!("Cannot interact with notifications"));
        };
        let conn = conn.clone();

        Ok(self.run_async(|request_id| async move {
            let result = show_notification_async(&conn, &summary, &body, sound_file_path).await;
            AsyncEventResult::NotificationShown { request_id, result }
        }))
    }

    pub fn request_close_notification(&mut self, notification_id: u32) -> anyhow::Result<()> {
        let Some(conn) = &self.state.notifications_connection else {
            return Err(anyhow!("Cannot interact with notifications"));
        };
        let conn = conn.clone();

        self.run_async(|_request_id| async move {
            if let Err(e) = close_notification_async(&conn, notification_id).await {
                warn!("Error closing notification {notification_id}: {e}");
            }
            AsyncEventResult::NotificationClosed {}
        });
        Ok(())
    }
}
