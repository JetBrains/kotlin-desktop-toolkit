use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::Result;
use desktop_common::ffi_utils::RustAllocatedRawPtr;
use desktop_common::logger::ffi_boundary;
use log::debug;
use smithay_client_toolkit::reexports::client::globals::GlobalList;
use smithay_client_toolkit::reexports::client::{Connection, globals::registry_queue_init};
use smithay_client_toolkit::reexports::client::{EventQueue, Proxy, QueueHandle};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::{
        WaylandSurface,
        xdg::{XdgShell, window::WindowDecorations},
    },
    shm::{Shm, slot::SlotPool},
    subcompositor::SubcompositorState,
};

use super::events::EventHandler;
use super::{application_state::ApplicationState, window::SimpleWindow};

#[repr(C)]
#[derive(Debug)]
pub struct ApplicationConfig {}

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
    state: ApplicationState,
}

impl Application {
    pub fn new(callbacks: ApplicationCallbacks) -> Result<Self> {
        let conn = Connection::connect_to_env()?;

        let (globals, event_queue) = registry_queue_init(&conn)?;
        let qh: QueueHandle<ApplicationState> = event_queue.handle();
        let state = Self::new_state(&globals, &qh, callbacks);
        Ok(Self {
            globals,
            event_queue,
            qh,
            exit: false,
            state,
        })
    }

    fn new_state(globals: &GlobalList, qh: &QueueHandle<ApplicationState>, callbacks: ApplicationCallbacks) -> ApplicationState {
        let registry_state = RegistryState::new(globals);
        let seat_state = SeatState::new(globals, qh);
        let output_state = OutputState::new(globals, qh);
        let compositor_state = CompositorState::bind(globals, qh).expect("wl_compositor not available");
        let shm_state = Shm::bind(globals, qh).expect("wl_shm not available");
        let xdg_shell_state = XdgShell::bind(globals, qh).expect("xdg shell not available");
        ApplicationState {
            callbacks,
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            shm_state,
            xdg_shell_state,
            keyboard: None,
            themed_pointer: None,
            windows: HashMap::new(),
            key_surface: None,
        }
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

    pub fn new_window(&mut self, event_handler: EventHandler) {
        let state = &self.state;
        let width = NonZeroU32::new(256).unwrap();
        let height = NonZeroU32::new(256).unwrap();
        let pool = SlotPool::new(width.get() as usize * height.get() as usize * 4, &state.shm_state).expect("Failed to create pool");

        let subcompositor_state = SubcompositorState::bind(state.compositor_state.wl_compositor().clone(), &self.globals, &self.qh)
            .expect("wl_subcompositor not available");
        let window_surface = state.compositor_state.create_surface(&self.qh);

        let window = state
            .xdg_shell_state
            .create_window(window_surface, WindowDecorations::ServerDefault, &self.qh);
        window.set_title("A wayland window");
        // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
        window.set_app_id("io.github.smithay.client-toolkit.SimpleWindow");
        window.set_min_size(Some((width.get(), height.get())));

        // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
        // For more info, see WaylandSurface::commit
        //
        // The compositor will respond with an initial configure that we can then use to present to the window with
        // the correct options.
        window.commit();

        let surface_id = window.wl_surface().id();

        debug!("Created new window with surface_id={surface_id}");
        let w = SimpleWindow {
            event_handler,
            subcompositor_state: Arc::new(subcompositor_state),
            close: false,
            first_configure: true,
            pool,
            width,
            height,
            buffer: None,
            window,
            window_frame: None,
            keyboard_focus: false,
            set_cursor: false,
            window_cursor_icon_idx: 0,
            decorations_cursor: None,
        };
        self.state.windows.insert(surface_id, w);
    }
}

pub type AppPtr<'a> = RustAllocatedRawPtr<'a, std::ffi::c_void>;

#[unsafe(no_mangle)]
pub extern "C" fn application_init(_config: &ApplicationConfig, callbacks: ApplicationCallbacks) -> AppPtr<'static> {
    let app = ffi_boundary("application_init", || {
        debug!("Application Init");
        // todo
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
