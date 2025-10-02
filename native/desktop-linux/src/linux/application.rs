use std::{ffi::CString, io::Read, str::FromStr, thread::ThreadId, time::Duration};

use anyhow::{Context, anyhow};
use log::{debug, warn};
use smithay_client_toolkit::{
    reexports::{
        calloop::{
            EventLoop, PostAction,
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

use crate::linux::{
    application_api::ApplicationCallbacks,
    application_state::ApplicationState,
    async_event_result::AsyncEventResult,
    data_transfer::MimeTypes,
    events::{DataTransferContent, DataTransferEvent, Event, RequestId, WindowId},
    window::SimpleWindow,
    window_api::WindowParams,
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

        WaylandSource::new(conn, event_queue)
            .insert(loop_handle)
            .map_err(|e| anyhow!(e.to_string()))?;

        let state = ApplicationState::new(&globals, &qh, callbacks, event_loop.handle());
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .worker_threads(1)
            .build()
            .unwrap();
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
        self.rt.spawn(xdg_desktop_settings_notifier(xdg_settings_sender));

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

    pub fn new_window(&mut self, params: &WindowParams) {
        let window_id = params.window_id;
        let w = SimpleWindow::new(window_id, &self.state, &self.qh, params);
        let surface_id = w.window.wl_surface().id();
        self.state.windows.insert(surface_id.clone(), w);
        self.state.window_id_to_surface_id.insert(window_id, surface_id);
    }

    pub fn get_window(&self, window_id: WindowId) -> anyhow::Result<&SimpleWindow> {
        self.state
            .window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.state.windows.get(surface_id))
            .with_context(|| format!("Couldn't find window for {window_id:?}"))
    }

    pub fn get_wl_surface(&self, window_id: WindowId) -> anyhow::Result<WlSurface> {
        Ok(self.get_window(window_id)?.window.wl_surface().clone())
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
            warn!("Application::clipboard_put: None");
            return;
        }
        let Some(device) = self.state.data_device.as_ref() else {
            warn!("Application::clipboard_put: No data device");
            return;
        };
        if let Some(serial) = self.state.last_implicit_grab_serial {
            let copy_paste_source = self
                .state
                .data_device_manager_state
                .create_copy_paste_source(&self.qh, mime_types.val);
            copy_paste_source.set_selection(device, serial);
            self.state.copy_paste_source = Some(copy_paste_source);
        } else {
            warn!("Application::clipboard_put: No last key down serial");
        }
    }

    pub fn clipboard_get_available_mimetypes(&self) -> Option<String> {
        let Some(data_device) = self.state.data_device.as_ref() else {
            warn!("Application::clipboard_paste: No data device available");
            return None;
        };
        let Some(selection_offer) = data_device.data().selection_offer() else {
            debug!("Application::clipboard_paste: No selection offer found");
            return None;
        };
        selection_offer.with_mime_types(|mime_types| Some(mime_types.join(",")))
    }

    pub fn primary_selection_put(&mut self, mime_types: MimeTypes) {
        if mime_types.val.is_empty() {
            self.state.primary_selection_source = None;
            warn!("Application::clipboard_put: None");
            return;
        }
        let Some(device) = self.state.primary_selection_device.as_ref() else {
            warn!("Application::clipboard_put: No primary selection device");
            return;
        };
        let Some(manager) = self.state.primary_selection_manager.as_ref() else {
            warn!("Application::clipboard_put: No primary selection manager");
            return;
        };
        if let Some(serial) = self.state.last_implicit_grab_serial {
            let source = manager.create_selection_source(&self.qh, mime_types.val);
            source.set_selection(device, serial);
            self.state.primary_selection_source = Some(source);
        } else {
            warn!("Application::clipboard_put: No last key down serial");
        }
    }

    pub fn primary_selection_paste(&self, serial: i32, supported_mime_types: &str) -> anyhow::Result<bool> {
        let Some(device) = self.state.primary_selection_device.as_ref() else {
            return Ok(false);
        };

        let Some(offer) = device.data().selection_offer() else {
            debug!("Application::primary_selection_paste: No selection offer found");
            return Ok(false);
        };
        let Some(mime_type) = offer.with_mime_types(|mime_types| {
            debug!("Application::primary_selection_paste: offer MIME types: {mime_types:?}, supported MIME types: {supported_mime_types}");
            supported_mime_types
                .split(',')
                .find(|&supported_mime_type| mime_types.iter().any(|m| m == supported_mime_type))
                .map(str::to_owned)
        }) else {
            debug!("Application::primary_selection_paste: clipboard content not supported");
            return Ok(false);
        };
        debug!("Application::primary_selection_paste reading {mime_type}");
        let read_pipe = offer.receive(mime_type.clone())?;
        self.event_loop.handle().insert_source(read_pipe, move |(), res, state| {
            let f = unsafe { res.get_mut() };
            let mut buf = Vec::new();
            let size = f.read_to_end(&mut buf).unwrap();

            debug!("Application::primary_selection_paste read {size} bytes");
            let mime_type_cstr = CString::from_str(&mime_type).unwrap();
            let content = DataTransferContent::new(&buf, &mime_type_cstr);
            state.send_event(DataTransferEvent { serial, content });

            PostAction::Remove
        })?;
        Ok(true)
    }

    pub fn clipboard_paste(&self, serial: i32, supported_mime_types: &str) -> anyhow::Result<bool> {
        let Some(device) = self.state.data_device.as_ref() else {
            warn!("Application::clipboard_paste: No data device available");
            return Ok(false);
        };
        let Some(offer) = device.data().selection_offer() else {
            debug!("Application::clipboard_paste: No selection offer found");
            return Ok(false);
        };
        let Some(mime_type) = offer.with_mime_types(|mime_types| {
            debug!("Application::clipboard_paste: offer MIME types: {mime_types:?}, supported MIME types: {supported_mime_types}");
            supported_mime_types
                .split(',')
                .find(|&supported_mime_type| mime_types.iter().any(|m| m == supported_mime_type))
                .map(str::to_owned)
        }) else {
            debug!("Application::clipboard_paste: clipboard content not supported");
            return Ok(false);
        };

        debug!("Application::clipboard_paste reading {mime_type}");
        let read_pipe = offer.receive(mime_type.clone())?;
        self.event_loop.handle().insert_source(read_pipe, move |(), res, state| {
            let f = unsafe { res.get_mut() };
            let mut buf = Vec::new();
            let size = f.read_to_end(&mut buf).unwrap();

            debug!("Application::clipboard_paste read {size} bytes");
            let mime_type_cstr = CString::from_str(&mime_type).unwrap();
            let content = DataTransferContent::new(&buf, &mime_type_cstr);
            state.send_event(DataTransferEvent { serial, content });

            PostAction::Remove
        })?;
        Ok(true)
    }

    pub fn start_drag(&mut self, window_id: WindowId, mime_types: MimeTypes, action: DndAction) -> anyhow::Result<()> {
        if mime_types.val.is_empty() {
            self.state.drag_source = None;
            return Ok(());
        }
        let w = self
            .get_window(window_id)
            .with_context(|| format!("No window found {window_id:?}"))?;
        let drag_source = self
            .state
            .data_device_manager_state
            .create_drag_and_drop_source(&self.qh, mime_types.val, action);
        let device = self.state.data_device.as_ref().context("No data device found")?;
        let origin = w.window.wl_surface();
        let serial = self
            .state
            .last_implicit_grab_serial
            .context("Called start_drag without an implicit grab")?;

        drag_source.start_drag(device, origin, None, serial); // TODO: icon
        self.state.current_drag_source_window_id = Some(window_id);
        self.state.drag_source = Some(drag_source);
        Ok(())
    }
}
