use crate::linux::events::ClipboardDataFFI;
use std::ffi::CStr;
use std::io::Read;
use std::time::Duration;

use super::application_api::ApplicationCallbacks;
use super::events::WindowId;
use super::geometry::LogicalSize;
use super::window_api::WindowParams;
use super::xdg_desktop_settings::xdg_desktop_settings_notifier;
use super::xdg_desktop_settings_api::XdgDesktopSetting;
use super::{application_state::ApplicationState, window::SimpleWindow};
use crate::linux::clipboard::{ClipboardContent, TEXT_MIME_TYPE, URI_LIST_MIME_TYPE};
use anyhow::anyhow;
use desktop_common::logger::catch_panic;
use log::{debug, warn};
use smithay_client_toolkit::reexports::calloop::{EventLoop, PostAction, channel};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    reexports::client::{Connection, Proxy, QueueHandle, globals::registry_queue_init},
    shell::WaylandSurface,
};

pub struct Application<'a> {
    pub event_loop: EventLoop<'a, ApplicationState>,
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
        debug!("Application event loop: starting");

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
                        self.state.window_id_to_surface_id.retain(|_window_id, surface_id| k != surface_id);
                    }
                    !v.close
                });
            }

            if self.exit && (self.state.callbacks.on_should_terminate)() {
                debug!("Exiting");
                (self.state.callbacks.on_will_terminate)();
                self.state.windows.clear();
                break;
            }
            // debug!("Application event loop: continuing");
        }
        debug!("Application event loop: stopped");
        Ok(())
    }

    pub fn new_window(&mut self, params: &WindowParams) {
        let window_id = params.window_id;
        let event_handler = self.state.callbacks.event_handler;
        let w = SimpleWindow::new(
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

    pub fn set_cursor_theme(&mut self, name: &str, size: u32) -> anyhow::Result<()> {
        self.state.set_cursor_theme(&self.qh, name, size)
    }

    pub fn clipboard_put(&mut self, clipboard_content: ClipboardContent) {
        self.state.clipboard_content = clipboard_content;
        if matches!(self.state.clipboard_content, ClipboardContent::None) {
            self.state.copy_paste_source = None;
            warn!("application_clipboard_put: None");
        } else if let Some(data_device) = self.state.data_device.as_ref() {
            if let Some(serial) = self.state.last_key_down_serial {
                let mime_types = self.state.clipboard_content.mime_types();
                let copy_paste_source = self.state.data_device_manager_state.create_copy_paste_source(&self.qh, mime_types);
                copy_paste_source.set_selection(data_device, serial);
                self.state.copy_paste_source = Some(copy_paste_source);
            } else {
                warn!("application_clipboard_put: No last key down serial");
            }
        } else {
            warn!("application_clipboard_put: No data device");
        }
    }

    pub fn clipboard_paste(&self) {
        if let Some(data_device) = self.state.data_device.as_ref() {
            if let Some(offer) = data_device.data().selection_offer() {
                offer.with_mime_types(|mime_types| {
                    debug!("application_clipboard_paste: offer MIME types: {mime_types:?}");
                    if mime_types.iter().any(|e| e == TEXT_MIME_TYPE) {
                        let is_uri_list = mime_types.iter().any(|e| e == URI_LIST_MIME_TYPE);
                        let read_pipe = offer
                            .receive(if is_uri_list { URI_LIST_MIME_TYPE } else { TEXT_MIME_TYPE }.to_owned())
                            .unwrap();
                        self.event_loop
                            .handle()
                            .insert_source(read_pipe, move |(), res, state| {
                                let f = unsafe { res.get_mut() };
                                let mut buf = Vec::new();
                                let size = f.read_to_end(&mut buf).unwrap();
                                buf.push(0);
                                let cstr = CStr::from_bytes_with_nul(&buf).unwrap();

                                debug!("application_clipboard_paste read {size} bytes");
                                if let Some(key_window) = state.get_key_window() {
                                    if is_uri_list {
                                        (key_window.event_handler)(&ClipboardDataFFI::new_file_list(cstr).into());
                                    } else {
                                        (key_window.event_handler)(&ClipboardDataFFI::new_string(cstr).into());
                                    }
                                } else {
                                    warn!("application_clipboard_paste: No key window");
                                }

                                PostAction::Remove
                            })
                            .unwrap();
                    }
                });
            } else {
                warn!("application_clipboard_paste: No selection offer found");
            }
        } else {
            warn!("application_clipboard_paste: No data device available");
        }
    }
}
