use std::collections::HashMap;

use log::debug;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry, delegate_seat, delegate_shm,
    delegate_subcompositor, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    reexports::client::{
        Connection, Proxy, QueueHandle,
        backend::ObjectId,
        protocol::{wl_keyboard, wl_output, wl_pointer::WlPointer, wl_seat, wl_surface::WlSurface},
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
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

pub struct ApplicationState {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub xdg_shell_state: XdgShell,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub themed_pointer: Option<ThemedPointer>,
    pub windows: HashMap<ObjectId, SimpleWindow>,
    pub key_surface: Option<ObjectId>,
}

struct WindowData<'a> {
    window: &'a mut SimpleWindow,
    shm: &'a Shm,
    themed_pointer: Option<&'a mut ThemedPointer>,
}

impl ApplicationState {
    fn get_window(&mut self, surface: &WlSurface) -> Option<&mut SimpleWindow> {
        let surface_id = &surface.id();
        debug!("Getting window for {surface_id}");
        self.windows.get_mut(surface_id)
    }

    fn get_window_data(&mut self, surface: &WlSurface) -> Option<WindowData> {
        self.windows.get_mut(&surface.id()).map(|window| WindowData {
            window,
            shm: &self.shm_state,
            themed_pointer: self.themed_pointer.as_mut(),
        })
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
        if let Some(window) = self.get_window(&surface) {
            window.keyboard_enter(keysyms);
        }
    }

    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, surface: &WlSurface, _: u32) {
        if let Some(window) = self.get_window(&surface) {
            window.keyboard_leave();
        }
        self.key_surface = None;
    }

    fn press_key(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, event: KeyEvent) {
        if let Some(surface_id) = self.key_surface.as_ref() {
            self.windows.get_mut(&surface_id).unwrap().press_key(event);
        }
    }

    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _: Modifiers,
        _layout: u32,
    ) {
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
            println!("Set keyboard capability");
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.themed_pointer.is_none() {
            println!("Set pointer capability");
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
                println!("Unset keyboard capability");
                keyboard.release();
            }
        }

        if capability == Capability::Pointer {
            if let Some(themed_pointer) = self.themed_pointer.take() {
                println!("Unset pointer capability");
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

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

delegate_output!(ApplicationState);

impl ShmHandler for ApplicationState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

delegate_shm!(ApplicationState);

impl CompositorHandler for ApplicationState {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _new_factor: i32) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Not needed for this example.
    }

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, _time: u32) {
        if let Some(window_data) = self.get_window_data(&surface) {
            window_data.window.draw(conn, qh, window_data.themed_pointer);
        }
    }

    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _output: &wl_output::WlOutput) {
        // Not needed for this example.
    }

    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _output: &wl_output::WlOutput) {
        // Not needed for this example.
    }
}

delegate_compositor!(ApplicationState);

impl WindowHandler for ApplicationState {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        if let Some(window) = self.get_window(&window.wl_surface()) {
            window.request_close();
        }
    }

    fn configure(&mut self, conn: &Connection, qh: &QueueHandle<Self>, window: &Window, configure: WindowConfigure, _serial: u32) {
        if let Some(window_data) = self.get_window_data(&window.wl_surface()) {
            window_data
                .window
                .configure(conn, qh, window_data.shm, window, configure, window_data.themed_pointer);
        }
    }
}

delegate_xdg_shell!(ApplicationState);

delegate_subcompositor!(ApplicationState);

impl PointerHandler for ApplicationState {
    fn pointer_frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, pointer: &WlPointer, events: &[PointerEvent]) {
        for event in events {
            debug!("pointer event with surface_id={}", event.surface.id());
            for window in self.windows.values_mut() {
                window.pointer_frame(pointer, event);
            }
        }
    }
}

delegate_pointer!(ApplicationState);
delegate_xdg_window!(ApplicationState);
