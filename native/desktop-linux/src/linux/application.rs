use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use anyhow::Result;
use desktop_common::logger::ffi_boundary;
use desktop_common::{ffi_utils::RustAllocatedRawPtr, logger::catch_panic};
use log::debug;
use smithay_client_toolkit::dmabuf::DmabufState;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::client::{
        Connection, EventQueue, Proxy, QueueHandle,
        globals::{GlobalList, registry_queue_init},
    },
    registry::RegistryState,
    seat::SeatState,
    shell::{
        WaylandSurface,
        xdg::{XdgShell, window::WindowDecorations},
    },
    shm::{Shm, slot::SlotPool},
    subcompositor::SubcompositorState,
};

use super::events::{EventHandler, LogicalPixels, LogicalSize, WindowId};
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

#[repr(C)]
pub struct WindowParams {
    //pub origin: LogicalPoint,
    pub width: u32,
    pub height: u32,
    //pub title: BorrowedStrPtr<'a>,

    //pub is_resizable: bool,
    //pub is_closable: bool,
    //pub is_miniaturizable: bool,

    //pub is_full_screen_allowed: bool,
    //pub use_custom_titlebar: bool,
    //pub titlebar_height: LogicalPixels,
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
        let dma_state = DmabufState::new(globals, qh);
        debug!("DMA-BUF protocol version: {:?}", dma_state.version());
        ApplicationState {
            callbacks,
            dma_state,
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            shm_state,
            xdg_shell_state,
            keyboard: None,
            themed_pointer: None,
            last_window_id: WindowId(0),
            window_id_to_surface_id: HashMap::new(),
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

    pub fn new_window(&mut self, event_handler: EventHandler, params: &WindowParams) -> WindowId {
        let state = &self.state;
        let width = NonZeroU32::new(params.width).unwrap();
        let height = NonZeroU32::new(params.height).unwrap();
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
            event_handler: {
                //let surface_id = surface_id.clone();
                Box::new(move |e| {
                    catch_panic(|| {
                        //debug!("Calling event handler of {surface_id} for {e:?}");
                        Ok(event_handler(e))
                    })
                    .unwrap_or(false)
                })
            },
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
    fn get_window(&self, window_id: WindowId) -> Option<&SimpleWindow> {
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
