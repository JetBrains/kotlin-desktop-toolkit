use anyhow::Result;
use desktop_common::logger::ffi_boundary;
use desktop_common::{ffi_utils::RustAllocatedRawPtr, logger::catch_panic};
use log::debug;
use smithay_client_toolkit::{
    reexports::client::{
        Connection, EventQueue, Proxy, QueueHandle,
        globals::{GlobalList, registry_queue_init},
    },
    shell::WaylandSurface,
};

use super::events::{EventHandler, LogicalPixels, LogicalSize, WindowId};
use super::window::WindowParams;
use super::{application_state::ApplicationState, window::SimpleWindow};

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationCallbacks {
    // Returns true if application should terminate, otherwise termination will be canceled
    pub on_should_terminate: extern "C" fn() -> bool,
    pub on_will_terminate: extern "C" fn(),
    pub on_display_configuration_change: extern "C" fn(),
}

pub struct Application {
    globals: GlobalList,
    event_queue: EventQueue<ApplicationState>,
    qh: QueueHandle<ApplicationState>,
    exit: bool,
    pub state: ApplicationState,
}

impl Application {
    pub fn new(callbacks: ApplicationCallbacks) -> Result<Self> {
        let conn = Connection::connect_to_env()?;

        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh: QueueHandle<ApplicationState> = event_queue.handle();
        let state = ApplicationState::new(&globals, &qh, callbacks);
        Ok(Self {
            globals,
            event_queue,
            qh,
            exit: false,
            state,
        })
    }

    pub fn run(&mut self) {
        debug!("Start event loop");
        loop {
            self.event_queue.blocking_dispatch(&mut self.state).unwrap();

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
        }
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
        app.run();
        Ok(())
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
