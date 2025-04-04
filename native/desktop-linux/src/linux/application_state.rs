use std::collections::HashMap;

use khronos_egl;
use log::{debug, warn};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    delegate_subcompositor, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    reexports::{
        client::{
            Connection, Dispatch, Proxy, QueueHandle,
            backend::ObjectId,
            delegate_noop,
            globals::GlobalList,
            protocol::{wl_keyboard, wl_output, wl_pointer::WlPointer, wl_seat, wl_surface::WlSurface},
        },
        protocols::wp::{
            fractional_scale::v1::client::{
                wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
                wp_fractional_scale_v1::{self, WpFractionalScaleV1},
            },
            viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardData, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerHandler, ThemeSpec, ThemedPointer},
    },
    shell::{
        WaylandSurface,
        xdg::{
            XdgShell,
            window::{Window, WindowConfigure, WindowHandler},
        },
    },
    shm::{Shm, ShmHandler},
};

use crate::linux::window::SimpleWindow;

use super::{application_api::ApplicationCallbacks, events::WindowId};

pub type EglInstance = khronos_egl::Instance<khronos_egl::Dynamic<libloading::Library, khronos_egl::EGL1_0>>;

pub struct ApplicationState {
    pub callbacks: ApplicationCallbacks,

    registry_state: RegistryState,
    seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub xdg_shell_state: XdgShell,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    themed_pointer: Option<ThemedPointer>,
    pub viewporter: Option<WpViewporter>,
    pub fractional_scale_manager: Option<WpFractionalScaleManagerV1>,

    pub last_window_id: WindowId,
    pub window_id_to_surface_id: HashMap<WindowId, ObjectId>,
    pub windows: HashMap<ObjectId, SimpleWindow>,
    key_surface: Option<ObjectId>,
    pub egl: Option<EglInstance>,
}

struct WindowWithData<'a> {
    window: &'a mut SimpleWindow,
    shm: &'a Shm,
    themed_pointer: Option<&'a mut ThemedPointer>,
    egl: Option<&'a EglInstance>,
}

impl ApplicationState {
    #[must_use]
    pub fn new(globals: &GlobalList, qh: &QueueHandle<Self>, callbacks: ApplicationCallbacks) -> Self {
        let registry_state = RegistryState::new(globals);
        let seat_state = SeatState::new(globals, qh);
        let output_state = OutputState::new(globals, qh);
        let compositor_state = CompositorState::bind(globals, qh).expect("wl_compositor not available");
        let shm_state = Shm::bind(globals, qh).expect("wl_shm not available");
        let xdg_shell_state = XdgShell::bind(globals, qh).expect("xdg shell not available");
        let egl = unsafe { libloading::Library::new("libEGL.so.1") }
            .map_err(|e| warn!("{e}"))
            .and_then(|lib| unsafe { EglInstance::load_required_from(lib) }.map_err(|e| warn!("{e}")))
            .ok();

        Self {
            callbacks,
            registry_state,
            seat_state,
            output_state,
            compositor_state,
            shm_state,
            xdg_shell_state,
            keyboard: None,
            themed_pointer: None,
            viewporter: globals.bind(qh, 1..=1, ()).ok(),
            fractional_scale_manager: globals.bind(qh, 1..=1, ()).ok(),
            last_window_id: WindowId(0),
            window_id_to_surface_id: HashMap::new(),
            windows: HashMap::new(),
            key_surface: None,
            egl,
        }
    }

    pub fn get_window(&mut self, surface: &WlSurface) -> Option<&mut SimpleWindow> {
        let surface_id: &ObjectId = &surface.id();
        self.windows.get_mut(surface_id)
    }

    fn get_window_with_data(&mut self, surface: &WlSurface) -> Option<WindowWithData> {
        self.windows.get_mut(&surface.id()).map(|window| WindowWithData {
            window,
            shm: &self.shm_state,
            themed_pointer: self.themed_pointer.as_mut(),
            egl: self.egl.as_ref(),
        })
    }

    fn get_key_window(&mut self) -> Option<&mut SimpleWindow> {
        self.key_surface.as_mut().and_then(|surface_id| self.windows.get_mut(surface_id))
    }
}

impl KeyboardHandler for ApplicationState {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        surface: &WlSurface,
        _: u32,
        _: &[u32],
        keysyms: &[Keysym],
    ) {
        self.key_surface = Some(surface.id());
        if let Some(window) = self.get_window(surface) {
            window.keyboard_enter(keysyms);
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, surface: &WlSurface, _: u32) {
        if let Some(window) = self.get_window(surface) {
            window.keyboard_leave();
        }
        self.key_surface = None;
    }

    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, keyboard: &wl_keyboard::WlKeyboard, serial: u32, event: KeyEvent) {
        if let Some(window) = self.get_key_window() {
            let frame_action = window.press_key(&event);
            if let Some(keyboard_data) = keyboard.data::<KeyboardData<Self>>() {
                let seat = keyboard_data.seat();
                window.frame_action(seat, serial, frame_action);
            }
        }
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        if let Some(window) = self.get_key_window() {
            window.release_key(&event);
        }
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _layout: u32,
    ) {
        if let Some(window) = self.get_key_window() {
            window.update_modifiers(modifiers);
        }
    }
}

delegate_keyboard!(ApplicationState);

impl SeatHandler for ApplicationState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            debug!("Set keyboard capability");
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.themed_pointer.is_none() {
            debug!("Set pointer capability");
            let surface = self.compositor_state.create_surface(qh);
            let themed_pointer = self
                .seat_state
                .get_pointer_with_theme(qh, &seat, self.shm_state.wl_shm(), surface, ThemeSpec::default())
                .expect("Failed to create pointer");
            self.themed_pointer.replace(themed_pointer);
        }
    }

    fn remove_capability(&mut self, _conn: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                debug!("Unset keyboard capability");
                keyboard.release();
            }
        }

        if capability == Capability::Pointer {
            if let Some(themed_pointer) = self.themed_pointer.take() {
                debug!("Unset pointer capability");
                themed_pointer.pointer().release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

delegate_seat!(ApplicationState);

impl ProvidesRegistryState for ApplicationState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

delegate_registry!(ApplicationState);

impl OutputHandler for ApplicationState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {
        (self.callbacks.on_display_configuration_change)();
    }

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {
        (self.callbacks.on_display_configuration_change)();
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {
        (self.callbacks.on_display_configuration_change)();
    }
}

delegate_output!(ApplicationState);

impl ShmHandler for ApplicationState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

delegate_shm!(ApplicationState);

impl CompositorHandler for ApplicationState {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, new_factor: i32) {
        if self.fractional_scale_manager.is_none() {
            debug!("scale_factor_changed for {surface:?}: {new_factor}");
            if let Some(window) = self.get_window(surface) {
                window.scale_changed(new_factor.into());
            }
        }
    }

    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, new_transform: wl_output::Transform) {
        debug!("transform_changed for {surface:?}: {new_transform:?}");
    }

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, _time: u32) {
        if let Some(window_data) = self.get_window_with_data(surface) {
            window_data.window.draw(conn, qh, window_data.themed_pointer, window_data.egl);
        }
    }

    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, output: &wl_output::WlOutput) {
        if let Some(window) = self.get_window(surface) {
            window.output_changed(output);
        }
    }

    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, output: &wl_output::WlOutput) {
        if let Some(window) = self.get_window(surface) {
            window.output_changed(output);
        }
    }
}

delegate_compositor!(ApplicationState);

impl WindowHandler for ApplicationState {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        if let Some(window) = self.get_window(window.wl_surface()) {
            window.request_close();
        }
    }

    fn configure(&mut self, conn: &Connection, qh: &QueueHandle<Self>, window: &Window, configure: WindowConfigure, _serial: u32) {
        if let Some(window_data) = self.get_window_with_data(window.wl_surface()) {
            if window_data
                .window
                .configure(conn, window_data.shm, window, &configure, window_data.egl)
            {
                // Initiate the first draw.
                window_data.window.draw(conn, qh, window_data.themed_pointer, window_data.egl);
            }
        }
    }
}

delegate_xdg_shell!(ApplicationState);

delegate_subcompositor!(ApplicationState);

impl PointerHandler for ApplicationState {
    fn pointer_frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, pointer: &WlPointer, events: &[PointerEvent]) {
        for event in events {
            if let Some(window) = self.get_window(&event.surface) {
                window.pointer_event(pointer, event);
            }
        }
    }
}

delegate_pointer!(ApplicationState);
delegate_xdg_window!(ApplicationState);

delegate_noop!(ApplicationState: ignore WpFractionalScaleManagerV1);
delegate_noop!(ApplicationState: ignore WpFractionalScaleV1);
delegate_noop!(ApplicationState: ignore WpViewporter);
delegate_noop!(ApplicationState: ignore WpViewport);

impl Dispatch<WpFractionalScaleV1, ObjectId> for ApplicationState {
    fn event(
        this: &mut Self,
        _: &WpFractionalScaleV1,
        event: <WpFractionalScaleV1 as Proxy>::Event,
        surface_id: &ObjectId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            debug!("wp_fractional_scale_v1::Event::PreferredScale: {scale}");
            if let Some(window) = this.windows.get_mut(surface_id) {
                window.scale_changed(f64::from(scale) / 120.0);
            };
        }
    }
}
