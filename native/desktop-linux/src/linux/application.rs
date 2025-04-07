use std::time::Duration;

use anyhow::anyhow;
use desktop_common::logger::catch_panic;
use log::debug;
use smithay_client_toolkit::reexports::calloop::{EventLoop, channel};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    reexports::client::{Connection, Proxy, QueueHandle, globals::registry_queue_init},
    shell::WaylandSurface,
};

use super::application_api::ApplicationCallbacks;
use super::events::{EventHandler, WindowId};
use super::geometry::LogicalSize;
use super::window_api::WindowParams;
use super::xdg_desktop_settings::xdg_desktop_settings_notifier;
use super::xdg_desktop_settings_api::XdgDesktopSetting;
use super::{application_state::ApplicationState, window::SimpleWindow};

pub struct Application<'a> {
    event_loop: EventLoop<'a, ApplicationState>,
    qh: QueueHandle<ApplicationState>,
    pub exit: bool,
    pub state: ApplicationState,
}

impl Application<'_> {
    pub fn new(callbacks: ApplicationCallbacks) -> anyhow::Result<Self> {
        let conn = Connection::connect_to_env()?;

        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh: QueueHandle<ApplicationState> = event_queue.handle();

        let event_loop = EventLoop::<ApplicationState>::try_new()?;
        let loop_handle = event_loop.handle();

        WaylandSource::new(conn, event_queue)
            .insert(loop_handle)
            .map_err(|e| anyhow!(e.to_string()))?;

        let state = ApplicationState::new(&globals, &qh, callbacks);
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

    pub fn run(&mut self) -> Result<(), anyhow::Error> {
        debug!("Start event loop");

        self.init_xdg_desktop_settings_notifier();
        self.init_run_on_event_loop();

        self.state.event_loop_thread_id = Some(std::thread::current().id());
        (self.state.callbacks.on_application_started)();

        loop {
            self.event_loop.dispatch(Duration::from_millis(16), &mut self.state)?;
            if !self.state.windows.is_empty() {
                self.state.windows.retain(|k, v| {
                    if v.close {
                        debug!("Closing window {k}");
                    }
                    !v.close
                });
                if self.state.windows.is_empty() {
                    self.exit = true;
                }
            }

            if self.exit && (self.state.callbacks.on_should_terminate)() {
                debug!("Exiting");
                (self.state.callbacks.on_will_terminate)();
                break;
            }
            // debug!("Continuing event loop");
        }
        Ok(())
    }

    pub fn new_window(&mut self, event_handler: EventHandler, params: &WindowParams) -> WindowId {
        let w = SimpleWindow::new(
            &self.state,
            &self.qh,
            Box::new(move |e| catch_panic(|| Ok(event_handler(e))).unwrap_or(false)),
            params,
        );
        let surface_id = w.window.wl_surface().id();
        self.state.windows.insert(surface_id.clone(), w);
        self.state.last_window_id.0 += 1;
        self.state.window_id_to_surface_id.insert(self.state.last_window_id, surface_id);
        self.state.last_window_id
    }

    pub fn drop_window(&mut self, window_id: WindowId) {
        if let Some(surface_id) = self.state.window_id_to_surface_id.remove(&window_id) {
            self.state.windows.remove(&surface_id);
        }
    }

    #[must_use]
    pub fn get_window(&self, window_id: WindowId) -> Option<&SimpleWindow> {
        self.state
            .window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.state.windows.get(surface_id))
    }

    #[must_use]
    pub fn get_window_mut(&mut self, window_id: WindowId) -> Option<&mut SimpleWindow> {
        self.state
            .window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.state.windows.get_mut(surface_id))
    }

    #[must_use]
    pub fn get_window_size(&self, window_id: WindowId) -> Option<LogicalSize> {
        self.get_window(window_id).and_then(|w| w.size)
    }
}
