use std::{ffi::CString, io::Read, str::FromStr, time::Duration};

use anyhow::{Context, anyhow};
use desktop_common::logger::catch_panic;
use log::{debug, warn};
use smithay_client_toolkit::{
    reexports::{
        calloop::{EventLoop, PostAction, channel},
        calloop_wayland_source::WaylandSource,
        client::{Connection, Proxy, QueueHandle, globals::registry_queue_init, protocol::wl_data_device_manager::DndAction},
    },
    shell::WaylandSurface,
};

use crate::linux::{
    application_api::ApplicationCallbacks,
    application_state::ApplicationState,
    data_transfer::MimeTypes,
    events::{DataTransferContent, WindowId},
    window::SimpleWindow,
    window_api::WindowParams,
    xdg_desktop_settings::xdg_desktop_settings_notifier,
    xdg_desktop_settings_api::XdgDesktopSetting,
};

pub struct Application<'a> {
    pub event_loop: EventLoop<'a, ApplicationState>,
    qh: QueueHandle<ApplicationState>,
    pub exit: bool,
    pub state: ApplicationState,
}

impl Application<'static> {
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
        Ok(Self {
            event_loop,
            qh,
            exit: false,
            state,
        })
    }

    fn init_xdg_desktop_settings_notifier(&self) {
        let (xdg_settings_sender, xdg_settings_channel) = channel::channel();
        async_std::task::spawn(xdg_desktop_settings_notifier(xdg_settings_sender));

        self.event_loop
            .handle()
            .insert_source(xdg_settings_channel, move |event, (), state| {
                if let channel::Event::Msg(e) = event {
                    XdgDesktopSetting::with(e, |s| (state.callbacks.on_xdg_desktop_settings_change)(s));
                }
            })
            .unwrap();
    }

    fn init_run_on_event_loop(&mut self) {
        let (s, c) = channel::channel();
        self.state.run_on_event_loop = Some(s);

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

        self.state.windows.retain(|k, v| {
            if v.close {
                debug!("Closing window {k}");
                self.state.window_id_to_surface_id.retain(|_window_id, surface_id| k != surface_id);
                false
            } else {
                true
            }
        });

        if self.exit && (self.state.callbacks.on_should_terminate)() {
            debug!("Exiting");
            (self.state.callbacks.on_will_terminate)();
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

        self.state.event_loop_thread_id = Some(std::thread::current().id());
        (self.state.callbacks.on_application_started)();

        while self.event_loop_iteration()? {
            // debug!("Application event loop: continuing");
        }
        debug!("Application event loop: stopped");
        Ok(())
    }

    pub fn new_window(&mut self, params: &WindowParams) {
        let window_id = params.window_id;
        let event_handler = self.state.callbacks.event_handler;
        let w = SimpleWindow::new(
            window_id,
            &self.state,
            &self.qh,
            Box::new(move |e| catch_panic(|| Ok(event_handler(e, window_id))).unwrap_or(false)),
            params,
        );
        let surface_id = w.window.wl_surface().id();
        self.state.windows.insert(surface_id.clone(), w);
        self.state.window_id_to_surface_id.insert(window_id, surface_id);
    }

    #[must_use]
    pub fn get_window(&self, window_id: WindowId) -> Option<&SimpleWindow> {
        self.state.get_window_by_id(window_id)
    }

    #[must_use]
    pub fn get_window_mut(&mut self, window_id: WindowId) -> Option<&mut SimpleWindow> {
        self.state
            .window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.state.windows.get_mut(surface_id))
    }

    pub fn set_cursor_theme(&mut self, name: &str, size: u32) -> anyhow::Result<()> {
        self.state.set_cursor_theme(&self.qh, name, size)
    }

    pub fn clipboard_put(&mut self, mime_types: MimeTypes) {
        if mime_types.val.is_empty() {
            self.state.copy_paste_source = None;
            warn!("Application::clipboard_put: None");
            return;
        };
        let Some(data_device) = self.state.data_device.as_ref() else {
            warn!("Application::clipboard_put: No data device");
            return;
        };
        if let Some(serial) = self.state.last_key_down_serial {
            let copy_paste_source = self
                .state
                .data_device_manager_state
                .create_copy_paste_source(&self.qh, mime_types.val);
            copy_paste_source.set_selection(data_device, serial);
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

    pub fn clipboard_paste(&self, window_id: WindowId, serial: i32, supported_mime_types: &str) -> anyhow::Result<bool> {
        let Some(data_device) = self.state.data_device.as_ref() else {
            warn!("Application::clipboard_paste: No data device available");
            return Ok(false);
        };
        let Some(selection_offer) = data_device.data().selection_offer() else {
            debug!("Application::clipboard_paste: No selection offer found");
            return Ok(false);
        };
        let Some(mime_type) = selection_offer.with_mime_types(|mime_types| {
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
        let read_pipe = selection_offer.receive(mime_type.clone())?;
        self.event_loop.handle().insert_source(read_pipe, move |(), res, state| {
            let f = unsafe { res.get_mut() };
            let mut buf = Vec::new();
            let size = f.read_to_end(&mut buf).unwrap();

            debug!("Application::clipboard_paste read {size} bytes");
            if let Some(target_window) = state.get_window_by_id(window_id) {
                let mime_type_cstr = CString::from_str(&mime_type).unwrap();
                (target_window.event_handler)(&DataTransferContent::new(serial, &buf, &mime_type_cstr).into());
            } else {
                warn!("Application::clipboard_paste: No target window");
            }

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
        let d = self.state.data_device.as_ref().context("No data device found")?;
        d.inner().start_drag(
            Some(drag_source.inner()),
            w.window.wl_surface(),
            None,
            w.current_mouse_down_serial.unwrap(),
        );
        self.state.drag_source = Some(drag_source);
        Ok(())
    }
}
