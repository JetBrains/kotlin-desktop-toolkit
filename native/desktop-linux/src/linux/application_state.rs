use std::{collections::HashMap, thread::ThreadId};

use khronos_egl;
use log::{debug, warn};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    data_device_manager::{
        DataDeviceManagerState,
        data_device::DataDevice,
        data_source::{CopyPasteSource, DragSource},
    },
    delegate_compositor, delegate_output, delegate_registry, delegate_seat, delegate_shm, delegate_subcompositor, delegate_xdg_shell,
    delegate_xdg_window,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{LoopHandle, channel::Sender},
        client::{
            Connection, Dispatch, Proxy, QueueHandle,
            backend::ObjectId,
            delegate_noop,
            globals::GlobalList,
            protocol::{
                wl_keyboard::WlKeyboard,
                wl_output::{self, WlOutput},
                wl_seat::WlSeat,
                wl_surface::WlSurface,
            },
        },
        protocols::wp::{
            fractional_scale::v1::client::{
                wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
                wp_fractional_scale_v1::{self, WpFractionalScaleV1},
            },
            text_input::zv3::client::{zwp_text_input_manager_v3::ZwpTextInputManagerV3, zwp_text_input_v3::ZwpTextInputV3},
            viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{PointerData, ThemeSpec, ThemedPointer},
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

use crate::linux::{application_api::ApplicationCallbacks, events::WindowId, text_input::PendingTextInputEvent, window::SimpleWindow};

pub type EglInstance = khronos_egl::Instance<khronos_egl::Dynamic<libloading::Library, khronos_egl::EGL1_0>>;

pub struct ApplicationState {
    pub callbacks: ApplicationCallbacks,

    registry_state: RegistryState,
    seat_state: SeatState,
    pub loop_handle: LoopHandle<'static, ApplicationState>,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub xdg_shell_state: XdgShell,
    keyboard: Option<WlKeyboard>,
    cursor_theme: Option<(String, u32)>,
    pub themed_pointer: Option<ThemedPointer>,
    pub viewporter: Option<WpViewporter>,
    pub fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    pub text_input_manager: Option<ZwpTextInputManagerV3>,
    pub data_device_manager_state: DataDeviceManagerState,
    pub copy_paste_source: Option<CopyPasteSource>,
    pub drag_source: Option<DragSource>,
    pub drag_destination_mime_type: Option<String>,
    pub data_device: Option<DataDevice>,

    pub window_id_to_surface_id: HashMap<WindowId, ObjectId>,
    pub windows: HashMap<ObjectId, SimpleWindow>,
    pub(crate) last_key_down_serial: Option<u32>,
    pub(crate) key_surface: Option<ObjectId>,
    pub(crate) active_text_input: Option<ZwpTextInputV3>,
    pub(crate) active_text_input_surface: Option<ObjectId>,
    pub(crate) pending_text_input_event: PendingTextInputEvent,
    pub egl: Option<EglInstance>,
    pub event_loop_thread_id: Option<ThreadId>,
    pub run_on_event_loop: Option<Sender<extern "C" fn()>>,
}

impl ApplicationState {
    #[must_use]
    pub fn new(
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        callbacks: ApplicationCallbacks,
        loop_handle: LoopHandle<'static, Self>,
    ) -> Self {
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
        let data_device_manager_state = DataDeviceManagerState::bind(globals, qh).expect("wl_data_device not available");

        Self {
            callbacks,
            registry_state,
            seat_state,
            loop_handle,
            output_state,
            compositor_state,
            shm_state,
            xdg_shell_state,
            keyboard: None,
            cursor_theme: None,
            themed_pointer: None,
            viewporter: globals.bind(qh, 1..=1, ()).ok(),
            fractional_scale_manager: globals.bind(qh, 1..=1, ()).ok(),
            text_input_manager: globals.bind(qh, 1..=1, ()).ok(),
            data_device_manager_state,
            copy_paste_source: None,
            drag_source: None,
            drag_destination_mime_type: None,
            data_device: None,
            window_id_to_surface_id: HashMap::new(),
            windows: HashMap::new(),
            last_key_down_serial: None,
            key_surface: None,
            active_text_input: None,
            active_text_input_surface: None,
            pending_text_input_event: PendingTextInputEvent::default(),
            egl,
            event_loop_thread_id: None,
            run_on_event_loop: None,
        }
    }

    pub fn get_window(&self, surface: &WlSurface) -> Option<&SimpleWindow> {
        let surface_id: &ObjectId = &surface.id();
        self.windows.get(surface_id)
    }

    pub fn get_window_by_id(&self, window_id: WindowId) -> Option<&SimpleWindow> {
        self.window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.windows.get(surface_id))
    }

    pub(crate) fn get_key_window(&self) -> Option<&SimpleWindow> {
        self.key_surface.as_ref().and_then(|surface_id| self.windows.get(surface_id))
    }

    fn update_themed_cursor_with_seat(&mut self, qh: &QueueHandle<Self>, seat: &WlSeat) -> anyhow::Result<()> {
        let theme = self
            .cursor_theme
            .as_ref()
            .map(|cursor_theme| ThemeSpec::Named {
                name: &cursor_theme.0,
                size: cursor_theme.1,
            })
            .unwrap_or_default();
        let surface = self.compositor_state.create_surface(qh);
        debug!("Created cursor surface {}", surface.id());

        let new_themed_pointer = self
            .seat_state
            .get_pointer_with_theme(qh, seat, self.shm_state.wl_shm(), surface, theme)?;
        self.themed_pointer = Some(new_themed_pointer);
        Ok(())
    }

    fn update_themed_cursor(&mut self, qh: &QueueHandle<Self>) -> anyhow::Result<()> {
        if let Some(themed_pointer) = self.themed_pointer.take() {
            let seat = themed_pointer.pointer().data::<PointerData>().unwrap().seat();
            self.update_themed_cursor_with_seat(qh, seat)?;
        }
        Ok(())
    }

    pub fn set_cursor_theme(&mut self, qh: &QueueHandle<Self>, name: &str, size: u32) -> anyhow::Result<()> {
        self.cursor_theme = Some((name.to_string(), size));
        self.update_themed_cursor(qh)
    }
}

impl SeatHandler for ApplicationState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {
        debug!("SeatHandler::new_seat");
    }

    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: WlSeat, capability: Capability) {
        if self.data_device.is_none() {
            self.data_device = Some(self.data_device_manager_state.get_data_device(qh, &seat));
        }

        if capability == Capability::Keyboard && self.keyboard.is_none() {
            debug!("Set keyboard capability");
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);

            if let Some(text_input_manager) = self.text_input_manager.as_ref() {
                text_input_manager.get_text_input(&seat, qh, 42);
            }
        }

        if capability == Capability::Pointer && self.themed_pointer.is_none() {
            debug!("Set pointer capability");
            self.update_themed_cursor_with_seat(qh, &seat).expect("Failed to create pointer");
        }
    }

    fn remove_capability(&mut self, _conn: &Connection, _: &QueueHandle<Self>, _: WlSeat, capability: Capability) {
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

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
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

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        (self.callbacks.on_display_configuration_change)();
    }

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        (self.callbacks.on_display_configuration_change)();
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
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
        debug!("scale_factor_changed for {}: {new_factor}", surface.id());
        if self.fractional_scale_manager.is_none() {
            if let Some(window) = self.windows.get_mut(&surface.id()) {
                window.scale_changed(new_factor.into(), &self.shm_state);
            }
        }
    }

    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, new_transform: wl_output::Transform) {
        debug!("transform_changed for {}: {new_transform:?}", surface.id());
    }

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, _time: u32) {
        if let Some(window) = self.windows.get_mut(&surface.id()) {
            window.draw(conn, qh, self.themed_pointer.as_mut(), self.egl.as_ref());
        }
    }

    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, output: &WlOutput) {
        debug!("surface_enter for {}: {}", surface.id(), output.id());
        if let Some(window) = self.get_window(surface) {
            //let screen_info = ScreenInfo::new(self.output_state.info(output));  // TODO?
            window.output_changed(output);
        }
    }

    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, output: &WlOutput) {
        debug!("surface_leave for {}: {}", surface.id(), output.id());
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
        if let Some(w) = self.windows.get_mut(&window.wl_surface().id()) {
            let egl = self.egl.as_ref();
            if w.configure(conn, &self.shm_state, window, &configure, egl) {
                // Initiate the first draw.
                w.draw(conn, qh, self.themed_pointer.as_mut(), egl);
            }
        }
    }
}

delegate_xdg_window!(ApplicationState);

delegate_xdg_shell!(ApplicationState);

delegate_subcompositor!(ApplicationState);

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
                window.scale_changed(f64::from(scale) / 120.0, &this.shm_state);
            };
        }
    }
}
