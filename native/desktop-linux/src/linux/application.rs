use std::time::Duration;

use desktop_common::logger::ffi_boundary;
use desktop_common::{ffi_utils::RustAllocatedRawPtr, logger::catch_panic};
use log::debug;
use smithay_client_toolkit::reexports::calloop::{EventLoop, channel};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::{
    reexports::client::{
        Connection, Proxy, QueueHandle,
        globals::{GlobalList, registry_queue_init},
    },
    shell::WaylandSurface,
};

use super::events::{EventHandler, LogicalPixels, LogicalSize, WindowId};
use super::window::WindowParams;
use super::xdg_desktop_settings::xdg_desktop_settings_notifier;
use super::{application_state::ApplicationState, window::SimpleWindow};

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    // Returns true if application should terminate, otherwise termination will be canceled
    pub on_should_terminate: extern "C" fn() -> bool,
    pub on_will_terminate: extern "C" fn(),
    pub on_display_configuration_change: extern "C" fn(),
}

pub struct Application<'a> {
    globals: GlobalList,
    event_loop: EventLoop<'a, ApplicationState>,
    qh: QueueHandle<ApplicationState>,
    exit: bool,
    pub state: ApplicationState,
}

impl Application<'_> {
    pub fn new(callbacks: ApplicationCallbacks) -> anyhow::Result<Self> {
        let conn = Connection::connect_to_env()?;

        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh: QueueHandle<ApplicationState> = event_queue.handle();

        let event_loop = EventLoop::<ApplicationState>::try_new()?;
        let loop_handle = event_loop.handle();

        WaylandSource::new(conn, event_queue).insert(loop_handle)?;

        let (s, c) = channel::channel();
        async_std::task::spawn(xdg_desktop_settings_notifier(s));

        event_loop
            .handle()
            .insert_source(c, |event, _a, state| {
                if let channel::Event::Msg(e) = event {
                    for w in state.windows.values_mut() {
                        w.handle_xdg_desktop_setting(&e);
                    }
                }
            })
            .unwrap();

        let state = ApplicationState::new(&globals, &qh, callbacks);
        Ok(Self {
            globals,
            event_loop,
            qh,
            exit: false,
            state,
        })
    }

    fn run(&mut self) -> Result<(), anyhow::Error> {
        debug!("Start event loop");

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
            &self.globals,
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
    pub fn get_window_size(&self, window_id: WindowId) -> Option<LogicalSize> {
        self.get_window(window_id).map(|w| LogicalSize {
            width: LogicalPixels(w.width.get().into()),
            height: LogicalPixels(w.height.get().into()),
        })
    }
}

pub type AppPtr<'a> = RustAllocatedRawPtr<'a, std::ffi::c_void>;

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
    ffi_boundary("application_stop_event_loop", || {
        debug!("Stop event loop");
        let app = unsafe { app_ptr.borrow_mut::<Application>() };
        app.exit = true;
        Ok(())
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn application_shutdown(app_ptr: AppPtr) {
    ffi_boundary("application_shutdown", || {
        let mut app = unsafe { app_ptr.to_owned::<Application>() };
        app.exit = true;
        Ok(())
    });
}
