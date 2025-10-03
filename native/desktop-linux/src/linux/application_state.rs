use std::{collections::HashMap, sync::LazyLock};

use desktop_common::logger::catch_panic;
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
    primary_selection::{PrimarySelectionManagerState, device::PrimarySelectionDevice, selection::PrimarySelectionSource},
    reexports::{
        calloop::LoopHandle,
        client::{
            Connection, Dispatch, Proxy, QueueHandle,
            backend::ObjectId,
            delegate_noop,
            globals::GlobalList,
            protocol::{
                wl_display::WlDisplay,
                wl_keyboard::WlKeyboard,
                wl_output::{self, WlOutput},
                wl_seat::WlSeat,
                wl_surface::WlSurface,
            },
        },
        csd_frame::WindowManagerCapabilities,
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

use crate::linux::{
    application_api::{ApplicationCallbacks, RenderingMode},
    events::{
        Event, ScreenId, WindowCapabilities, WindowCloseRequestEvent, WindowConfigureEvent, WindowDrawEvent, WindowId,
        WindowScaleChangedEvent, WindowScreenChangeEvent,
    },
    keyboard::send_key_down_event,
    text_input::PendingTextInputEvent,
    window::SimpleWindow,
};

/// cbindgen:ignore
pub type EglInstance = khronos_egl::DynamicInstance<khronos_egl::EGL1_0>;

/// cbindgen:ignore
static EGL: LazyLock<Option<EglInstance>> = LazyLock::new(|| match unsafe { libloading::Library::new("libEGL.so.1") } {
    Ok(egl_lib) => match unsafe { EglInstance::load_required_from(egl_lib) } {
        Ok(egl) => Some(egl),
        Err(e) => {
            warn!("Failed to load the required symbols from the EGL library: {e}");
            None
        }
    },
    Err(e) => {
        warn!("Failed to load EGL: {e}");
        None
    }
});

pub fn get_egl() -> Option<&'static EglInstance> {
    match &*EGL {
        Some(v) => Some(v),
        None => None,
    }
}

pub struct ApplicationState {
    pub callbacks: ApplicationCallbacks,

    registry_state: RegistryState,
    seat_state: SeatState,
    pub loop_handle: LoopHandle<'static, ApplicationState>,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub xdg_shell_state: XdgShell,
    pub wl_display: WlDisplay,
    pub keyboard: Option<WlKeyboard>,
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
    pub primary_selection_manager: Option<PrimarySelectionManagerState>,
    pub primary_selection_device: Option<PrimarySelectionDevice>,
    pub primary_selection_source: Option<PrimarySelectionSource>,

    pub window_id_to_surface_id: HashMap<WindowId, ObjectId>,
    pub windows: HashMap<ObjectId, SimpleWindow>,
    pub last_keyboard_event_serial: Option<u32>,
    pub active_text_input: Option<ZwpTextInputV3>,
    pub pending_text_input_event: PendingTextInputEvent,
}

impl ApplicationState {
    #[must_use]
    pub fn new(
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        callbacks: ApplicationCallbacks,
        loop_handle: LoopHandle<'static, Self>,
        display: WlDisplay,
    ) -> Self {
        let registry_state = RegistryState::new(globals);
        let seat_state = SeatState::new(globals, qh);
        let output_state = OutputState::new(globals, qh);
        let compositor_state = CompositorState::bind(globals, qh).expect("wl_compositor not available");
        let shm_state = Shm::bind(globals, qh).expect("wl_shm not available");
        let xdg_shell_state = XdgShell::bind(globals, qh).expect("xdg shell not available");
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
            wl_display: display,
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
            primary_selection_manager: PrimarySelectionManagerState::bind(globals, qh).ok(),
            primary_selection_device: None,
            primary_selection_source: None,
            window_id_to_surface_id: HashMap::new(),
            windows: HashMap::new(),
            last_keyboard_event_serial: None,
            active_text_input: None,
            pending_text_input_event: PendingTextInputEvent::default(),
        }
    }

    pub fn get_window_id(&self, surface: &WlSurface) -> Option<WindowId> {
        let surface_id: &ObjectId = &surface.id();
        self.windows.get(surface_id).map(|w| w.window_id)
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

    pub fn send_event<'a, T: Into<Event<'a>>>(&self, event_data: T) -> bool {
        let event: Event = event_data.into();
        self.callbacks.send_event(event)
    }

    pub fn get_latest_pointer_button_seat_and_serial(&self) -> Option<(&WlSeat, u32)> {
        if let Some(p) = self.themed_pointer.as_ref()
            && let Some(d) = p.pointer().data::<PointerData>()
            && let Some(s) = d.latest_button_serial()
        {
            Some((d.seat(), s))
        } else {
            None
        }
    }

    pub fn get_latest_event_serial(&self) -> Option<u32> {
        [
            self.get_latest_pointer_button_seat_and_serial().map(|e| e.1),
            self.last_keyboard_event_serial,
        ]
        .into_iter()
        .max()
        .flatten()
    }
}

impl ApplicationCallbacks {
    #[allow(clippy::needless_pass_by_value)]
    fn send_event(&self, event: Event) -> bool {
        match event {
            Event::MouseMoved(_) | Event::WindowDraw(_) => {}
            _ => debug!("Sending event: {event:?}"),
        }
        catch_panic(|| Ok((self.event_handler)(&event))).unwrap_or(false)
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

            self.primary_selection_device = self.primary_selection_manager.as_ref().map(|m| m.get_selection_device(qh, &seat));
        }

        if capability == Capability::Keyboard && self.keyboard.is_none() {
            debug!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|state, wl_kbd, event| {
                        // Since wl_keyboard version 10, [smithay_client_toolkit::seat::keyboard::KeyboardHandler::repeat_key]
                        // is used instead.
                        if wl_kbd.version() < 10 {
                            send_key_down_event(state, event, true);
                        }
                    }),
                )
                .expect("Failed to create keyboard");
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
        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            debug!("Unset keyboard capability");
            keyboard.release();
        }

        if capability == Capability::Pointer
            && let Some(themed_pointer) = self.themed_pointer.take()
        {
            debug!("Unset pointer capability");
            themed_pointer.pointer().release();
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
        self.send_event(Event::DisplayConfigurationChange);
    }

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        self.send_event(Event::DisplayConfigurationChange);
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: WlOutput) {
        self.send_event(Event::DisplayConfigurationChange);
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
        if self.fractional_scale_manager.is_none()
            && let Some(window) = self.windows.get_mut(&surface.id())
        {
            let new_scale: f64 = new_factor.into();
            window.scale_changed(new_scale, &self.shm_state);

            self.callbacks.send_event(
                WindowScaleChangedEvent {
                    window_id: window.window_id,
                    new_scale,
                }
                .into(),
            );
        }
    }

    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, new_transform: wl_output::Transform) {
        debug!("transform_changed for {}: {new_transform:?}", surface.id());
        // TODO?
    }

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, _time: u32) {
        if let Some(window) = self.windows.get_mut(&surface.id()) {
            window.draw(conn, qh, self.themed_pointer.as_mut(), &|e: WindowDrawEvent| {
                self.callbacks.send_event(e.into())
            });
        }
    }

    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, output: &WlOutput) {
        debug!("surface_enter for {}: {}", surface.id(), output.id());
        if let Some(window_id) = self.get_window_id(surface)
            && let Some(output_info) = self.output_state.info(output)
        {
            self.send_event(WindowScreenChangeEvent {
                window_id,
                new_screen_id: ScreenId(output_info.id),
            });
        }
    }

    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, output: &WlOutput) {
        debug!("surface_leave for {}: {}", surface.id(), output.id());
    }
}

delegate_compositor!(ApplicationState);

impl WindowHandler for ApplicationState {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        if let Some(window_id) = self.get_window_id(window.wl_surface()) {
            self.send_event(WindowCloseRequestEvent { window_id });
        }
    }

    fn configure(&mut self, conn: &Connection, qh: &QueueHandle<Self>, window: &Window, configure: WindowConfigure, _serial: u32) {
        if let Some(w) = self.windows.get_mut(&window.wl_surface().id()) {
            let egl = match w.rendering_mode {
                RenderingMode::Auto | RenderingMode::EGL => get_egl(),
                RenderingMode::Software => None,
            };
            let is_first_configure = w.configure(&self.wl_display, &self.shm_state, window, &configure, egl);

            self.callbacks.send_event(
                WindowConfigureEvent {
                    window_id: w.window_id,
                    size: w.size.unwrap(),
                    active: configure.is_activated(),
                    maximized: configure.is_maximized(),
                    fullscreen: configure.is_fullscreen(),
                    decoration_mode: configure.decoration_mode.into(),
                    capabilities: WindowCapabilities {
                        window_menu: configure.capabilities.contains(WindowManagerCapabilities::WINDOW_MENU),
                        maximize: configure.capabilities.contains(WindowManagerCapabilities::MAXIMIZE),
                        fullscreen: configure.capabilities.contains(WindowManagerCapabilities::FULLSCREEN),
                        minimize: configure.capabilities.contains(WindowManagerCapabilities::MINIMIZE),
                    },
                }
                .into(),
            );

            if is_first_configure {
                // Initiate the first draw.
                w.draw(conn, qh, self.themed_pointer.as_mut(), &|e: WindowDrawEvent| {
                    self.callbacks.send_event(e.into())
                });
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
        state: &mut Self,
        _: &WpFractionalScaleV1,
        event: <WpFractionalScaleV1 as Proxy>::Event,
        surface_id: &ObjectId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            let new_scale = f64::from(scale) / 120.0;
            debug!("wp_fractional_scale_v1::Event::PreferredScale: {scale}/120 ({new_scale})");
            if let Some(window) = state.windows.get_mut(surface_id) {
                window.scale_changed(new_scale, &state.shm_state);

                state.callbacks.send_event(
                    WindowScaleChangedEvent {
                        window_id: window.window_id,
                        new_scale,
                    }
                    .into(),
                );
            }
        }
    }
}
