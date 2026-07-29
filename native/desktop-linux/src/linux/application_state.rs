use crate::linux::application::send_event;
use crate::linux::ffi_return_conversions::{QueryDragAndDropTarget, TransferDataGetter};
use crate::linux::notifications::NotificationAction;
use crate::linux::{
    application_api::{ApplicationCallbacks, RenderingMode},
    drag_icon::DragIcon,
    events::{
        ActivationTokenResponse,
        Event,
        EventSerial,
        RequestId,
        ScreenId,
        WindowCapabilities,
        WindowCloseRequestEvent,
        WindowConfigureEvent,
        WindowDecorationMode,
        WindowDrawEvent,
        WindowFrame,
        WindowId,
        WindowScaleChangedEvent,
        WindowScreenChangeEvent,
        //
    },
    geometry::Scale,
    keyboard::send_key_down_event,
    text_input::PendingTextInputEvent,
    window::SimpleWindow,
};
use anyhow::Context;
use khronos_egl as egl;
use log::{debug, info, warn};
use smithay_client_toolkit::{
    activation::{ActivationHandler, ActivationState, RequestData},
    compositor::{CompositorHandler, CompositorState},
    data_device_manager::{
        DataDeviceManagerState,
        data_device::DataDevice,
        data_source::{CopyPasteSource, DragSource},
    },
    delegate_dispatch2,
    delegate_registry,
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
                wl_data_device_manager::DndAction,
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
        keyboard::KeyboardData,
        pointer::{PointerData, ThemeSpec, ThemedPointer},
    },
    shell::{
        WaylandSurface,
        xdg::{
            XdgShell,
            window::{DecorationMode, Window, WindowConfigure, WindowHandler},
        },
    },
    shm::{Shm, ShmHandler},
    //
};
use std::collections::HashMap;
use std::rc::Rc;

/// cbindgen:ignore
pub type EglInstance = khronos_egl::DynamicInstance<khronos_egl::EGL1_0>;

pub struct EGLData {
    pub instance: EglInstance,
    pub display: egl::Display,
}

fn init_egl(display: &WlDisplay) -> anyhow::Result<Rc<EGLData>> {
    let egl_instance = unsafe { EglInstance::load_required() }?;
    let wayland_display_ptr = display.id().as_ptr();
    let egl_display = unsafe { egl_instance.get_display(wayland_display_ptr.cast()) }.context("egl.get_display")?;
    egl_instance.initialize(egl_display).context("egl.initialize")?;

    Ok(Rc::new(EGLData {
        instance: egl_instance,
        display: egl_display,
    }))
}

fn get_egl(egl: &mut Option<anyhow::Result<Rc<EGLData>>>, display: &WlDisplay) -> Option<Rc<EGLData>> {
    egl.get_or_insert_with(|| {
        init_egl(display).map_err(|e| {
            warn!("Failed to load EGL: {e}");
            e
        })
    })
    .as_ref()
    .ok()
    .cloned()
}

impl Drop for EGLData {
    fn drop(&mut self) {
        if self.display.as_ptr() != egl::NO_DISPLAY {
            self.instance.terminate(self.display).expect("eglTerminate");
        }
    }
}

pub struct ApplicationState {
    pub transfer_data_getter: TransferDataGetter,
    pub query_drag_and_drop_target: QueryDragAndDropTarget,
    pub callbacks: ApplicationCallbacks,

    registry_state: RegistryState,
    seat_state: SeatState,
    pub loop_handle: LoopHandle<'static, Self>,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub xdg_shell_state: XdgShell,
    pub xdg_activation: Option<ActivationState>,
    wl_display: WlDisplay,
    pub keyboard: Option<WlKeyboard>,
    cursor_theme: Option<(String, u32)>,
    pub themed_pointer: Option<ThemedPointer>,
    pub viewporter: Option<WpViewporter>,
    pub fractional_scale_manager: Option<WpFractionalScaleManagerV1>,
    pub text_input_manager: Option<ZwpTextInputManagerV3>,
    pub data_device_manager_state: DataDeviceManagerState,
    pub copy_paste_source: Option<CopyPasteSource>,
    pub drag_source: Option<DragSource>,
    pub current_drag_target_window_id: Option<WindowId>,
    pub current_drag_source_window_id: Option<WindowId>,
    pub current_drag_source_action: Option<DndAction>,
    pub drag_icon: Option<DragIcon>,
    pub data_device: Option<DataDevice>,
    pub primary_selection_manager: Option<PrimarySelectionManagerState>,
    pub primary_selection_device: Option<PrimarySelectionDevice>,
    pub primary_selection_source: Option<PrimarySelectionSource>,

    egl: Option<anyhow::Result<Rc<EGLData>>>,
    pub window_id_to_surface_id: HashMap<WindowId, ObjectId>,
    pub windows: HashMap<ObjectId, SimpleWindow>,
    pub last_pointer_down_event_serial: Option<u32>,
    pub last_keyboard_event_serial: Option<u32>,
    pub active_text_input: Option<ZwpTextInputV3>,
    pub pending_text_input_event: PendingTextInputEvent,
    pub notification_action_sender: Option<tokio::sync::mpsc::Sender<NotificationAction>>,
    pub calloop_scheduler: calloop::futures::Scheduler<()>,
}

impl ApplicationState {
    pub fn new(
        globals: &GlobalList,
        qh: &QueueHandle<Self>,
        callbacks: ApplicationCallbacks,
        loop_handle: LoopHandle<'static, Self>,
        display: WlDisplay,
    ) -> anyhow::Result<Self> {
        let registry_state = RegistryState::new(globals);
        let seat_state = SeatState::new(globals, qh);
        let output_state = OutputState::new(globals, qh);
        let compositor_state = CompositorState::bind(globals, qh).expect("wl_compositor not available");
        let shm_state = Shm::bind(globals, qh).expect("wl_shm not available");
        let xdg_shell_state = XdgShell::bind(globals, qh).expect("xdg shell not available");
        let data_device_manager_state = DataDeviceManagerState::bind(globals, qh).expect("wl_data_device not available");
        let xdg_activation = ActivationState::bind(globals, qh).ok();

        let ffi_dealloc = callbacks.obj_dealloc;

        let (calloop_exec, calloop_scheduler) = calloop::futures::executor()?;
        loop_handle
            .insert_source(calloop_exec, |(), _metadata, _shared| {})
            .map_err(|e| e.error)?;

        Ok(Self {
            transfer_data_getter: TransferDataGetter {
                ffi_get: callbacks.get_data_transfer_data,
                ffi_dealloc,
            },
            query_drag_and_drop_target: QueryDragAndDropTarget {
                ffi_get: callbacks.query_drag_and_drop_target,
                ffi_dealloc,
            },
            callbacks,
            registry_state,
            seat_state,
            loop_handle,
            output_state,
            compositor_state,
            shm_state,
            xdg_shell_state,
            xdg_activation,
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
            current_drag_target_window_id: None,
            current_drag_source_window_id: None,
            current_drag_source_action: None,
            drag_icon: None,
            data_device: None,
            primary_selection_manager: PrimarySelectionManagerState::bind(globals, qh).ok(),
            primary_selection_device: None,
            primary_selection_source: None,
            egl: None,
            window_id_to_surface_id: HashMap::new(),
            windows: HashMap::new(),
            last_pointer_down_event_serial: None,
            last_keyboard_event_serial: None,
            active_text_input: None,
            pending_text_input_event: PendingTextInputEvent::default(),
            notification_action_sender: None,
            calloop_scheduler,
        })
    }

    pub fn get_egl(&mut self) -> Option<Rc<EGLData>> {
        get_egl(&mut self.egl, &self.wl_display)
    }

    pub fn get_window_id(&self, surface: &WlSurface) -> Option<WindowId> {
        let surface_id: &ObjectId = &surface.id();
        self.windows.get(surface_id).map(|w| w.window_id)
    }

    pub fn get_window(&self, window_id: WindowId) -> anyhow::Result<&SimpleWindow> {
        self.window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.windows.get(surface_id))
            .with_context(|| format!("Couldn't find window for {window_id:?}"))
    }

    pub fn get_window_mut(&mut self, window_id: WindowId) -> anyhow::Result<&mut SimpleWindow> {
        self.window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.windows.get_mut(surface_id))
            .with_context(|| format!("Couldn't find window for {window_id:?}"))
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
            .get_pointer_with_theme::<_, ()>(qh, seat, self.shm_state.wl_shm(), surface, theme)?;
        self.themed_pointer = Some(new_themed_pointer);
        Ok(())
    }

    fn update_themed_cursor(&mut self, qh: &QueueHandle<Self>) -> anyhow::Result<()> {
        if let Some(themed_pointer) = self.themed_pointer.take() {
            let seat = themed_pointer.pointer().data::<PointerData<()>>().unwrap().seat();
            self.update_themed_cursor_with_seat(qh, seat)?;
        }
        Ok(())
    }

    pub fn set_cursor_theme(&mut self, qh: &QueueHandle<Self>, name: &str, size: u32) -> anyhow::Result<()> {
        self.cursor_theme = Some((name.to_string(), size));
        self.update_themed_cursor(qh)
    }

    pub fn send_event<'a, T: Into<Event<'a>>>(&self, event_data: T) -> bool {
        send_event(self.callbacks.event_handler, event_data)
    }

    pub fn get_default_seat(&self) -> Option<WlSeat> {
        self.seat_state.seats().next()
    }

    pub fn get_latest_pointer_button_seat_and_serial(&self) -> Option<(&WlSeat, u32)> {
        if let Some(p) = &self.themed_pointer
            && let Some(pointer_data) = p.pointer().data::<PointerData<()>>()
            && let Some(pointer_event_serial) = self.last_pointer_down_event_serial
        {
            Some((pointer_data.seat(), pointer_event_serial))
        } else {
            None
        }
    }

    pub fn get_latest_event_seat_and_serial(&self) -> Option<(&WlSeat, u32)> {
        let pointer_event_seat_and_serial = self.get_latest_pointer_button_seat_and_serial();
        if let Some(keyboard_event_serial) = self.last_keyboard_event_serial
            && pointer_event_seat_and_serial.is_none_or(|(_, serial)| keyboard_event_serial > serial)
            && let Some(keyboard) = &self.keyboard
            && let Some(keyboard_data) = keyboard.data::<KeyboardData<Self, ()>>()
        {
            debug!("Using keyboard event serial");
            Some((keyboard_data.seat(), keyboard_event_serial))
        } else if let Some(pointer_event_seat_and_serial) = pointer_event_seat_and_serial {
            debug!("Using pointer event serial");
            Some(pointer_event_seat_and_serial)
        } else {
            None
        }
    }

    pub fn set_window_client_side_decoration_frame(&mut self, window_id: WindowId, frame: WindowFrame) -> anyhow::Result<()> {
        let w = self
            .window_id_to_surface_id
            .get(&window_id)
            .and_then(|surface_id| self.windows.get_mut(surface_id))
            .with_context(|| format!("Couldn't find window for {window_id:?}"))?;
        w.default_client_side_decoration_frame = frame.clone();

        if let Some(mut event) = w.last_configure_event.clone()
            && matches!(event.decoration_mode, WindowDecorationMode::Client(_))
            && !event.maximized
            && !event.fullscreen
        {
            w.set_client_side_decoration_frame(&frame, &self.compositor_state, &self.shm_state);

            event.decoration_mode = WindowDecorationMode::Client(frame);
            event.size = w.surface_size.unwrap();

            w.last_configure_event = Some(event.clone());
            _ = send_event(self.callbacks.event_handler, event);
        }
        Ok(())
    }
}

impl SeatHandler for ApplicationState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, seat: WlSeat) {
        debug!("SeatHandler::new_seat: {}", seat.id());
    }

    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: WlSeat, capability: Capability) {
        if self.data_device.is_none() {
            self.data_device = Some(self.data_device_manager_state.get_data_device(qh, &seat));

            self.primary_selection_device = self.primary_selection_manager.as_ref().map(|m| m.get_selection_device(qh, &seat));
        }

        debug!("New capability for seat {}: {capability:?}", seat.id());

        if capability == Capability::Keyboard && self.keyboard.is_none() {
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
                            send_key_down_event(state, &event, EventSerial(0), true);
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
            self.update_themed_cursor_with_seat(qh, &seat).expect("Failed to create pointer");
        }
    }

    fn remove_capability(&mut self, _conn: &Connection, _: &QueueHandle<Self>, seat: WlSeat, capability: Capability) {
        debug!("Remove capability for seat {}: {capability:?}", seat.id());

        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            keyboard.release();
        }

        if capability == Capability::Pointer
            && let Some(themed_pointer) = self.themed_pointer.take()
        {
            themed_pointer.pointer().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

delegate_dispatch2!(ApplicationState);

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

impl ShmHandler for ApplicationState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl CompositorHandler for ApplicationState {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, new_factor: i32) {
        debug!("scale_factor_changed for {}: {new_factor}", surface.id());
        if self.fractional_scale_manager.is_some() {
            return;
        }
        let new_scale = Scale::from_scale_factor(new_factor);
        if let Some(window) = self.windows.get_mut(&surface.id()) {
            window.scale_changed(new_scale, &self.shm_state);

            _ = send_event(
                self.callbacks.event_handler,
                WindowScaleChangedEvent {
                    window_id: window.window_id,
                    new_scale,
                },
            );
        } else if let Some(drag_icon) = &mut self.drag_icon
            && drag_icon.surface.wl_surface() == surface
        {
            drag_icon.scale_changed(new_scale, &self.shm_state);
        }
    }

    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, surface: &WlSurface, new_transform: wl_output::Transform) {
        debug!("transform_changed for {}: {new_transform:?}", surface.id());
        // TODO?
    }

    fn frame(&mut self, conn: &Connection, qh: &QueueHandle<Self>, surface: &WlSurface, _time: u32) {
        if let Some(window) = self.windows.get_mut(&surface.id()) {
            window.draw(conn, qh, self.themed_pointer.as_mut(), &|e: WindowDrawEvent| {
                send_event(self.callbacks.event_handler, e)
            });
        } else if let Some(drag_icon) = &mut self.drag_icon
            && drag_icon.surface.wl_surface() == surface
        {
            drag_icon.draw(qh, &|e| send_event(self.callbacks.event_handler, e));
        } else {
            warn!("Draw surface {} is neither a window nor a drag icon", surface.id());
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

impl WindowHandler for ApplicationState {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, window: &Window) {
        if let Some(window_id) = self.get_window_id(window.wl_surface()) {
            self.send_event(WindowCloseRequestEvent { window_id });
        }
    }

    fn configure(&mut self, conn: &Connection, qh: &QueueHandle<Self>, window: &Window, configure: WindowConfigure, _serial: u32) {
        if let Some(w) = self.windows.get_mut(&window.wl_surface().id()) {
            let egl = match w.rendering_mode {
                RenderingMode::Auto | RenderingMode::EGL => get_egl(&mut self.egl, &self.wl_display),
                RenderingMode::Software => None,
            };

            let mut frame = if configure.decoration_mode == DecorationMode::Server || configure.is_maximized() || configure.is_fullscreen()
            {
                WindowFrame::default()
            } else {
                w.default_client_side_decoration_frame.clone()
            };
            frame.left.tiled = configure.is_tiled_left();
            frame.right.tiled = configure.is_tiled_right();
            frame.top.tiled = configure.is_tiled_top();
            frame.bottom.tiled = configure.is_tiled_bottom();

            let is_first_configure = w.configure(&self.shm_state, &self.compositor_state, window, &configure, egl, &frame);

            let decoration_mode = match configure.decoration_mode {
                DecorationMode::Client => WindowDecorationMode::Client(frame),
                DecorationMode::Server => WindowDecorationMode::Server,
            };

            let event = WindowConfigureEvent {
                window_id: w.window_id,
                size: w.surface_size.unwrap(),
                active: configure.is_activated(),
                maximized: configure.is_maximized(),
                fullscreen: configure.is_fullscreen(),
                decoration_mode,
                capabilities: WindowCapabilities {
                    window_menu: configure.capabilities.contains(WindowManagerCapabilities::WINDOW_MENU),
                    maximize: configure.capabilities.contains(WindowManagerCapabilities::MAXIMIZE),
                    fullscreen: configure.capabilities.contains(WindowManagerCapabilities::FULLSCREEN),
                    minimize: configure.capabilities.contains(WindowManagerCapabilities::MINIMIZE),
                },
            };

            if w.last_configure_event.replace(event.clone()).is_none_or(|e| e != event) {
                _ = send_event(self.callbacks.event_handler, event);
            }

            if is_first_configure {
                // Initiate the first draw.
                w.draw(conn, qh, self.themed_pointer.as_mut(), &|e: WindowDrawEvent| {
                    send_event(self.callbacks.event_handler, e)
                });
            }
        }
    }
}

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
            let scale = Scale::from_value120(scale);
            debug!("wp_fractional_scale_v1::Event::PreferredScale: {scale:?}");
            if let Some(window) = state.windows.get_mut(surface_id) {
                window.scale_changed(scale, &state.shm_state);

                _ = send_event(
                    state.callbacks.event_handler,
                    WindowScaleChangedEvent {
                        window_id: window.window_id,
                        new_scale: scale,
                    },
                );
            } else if let Some(drag_icon) = &mut state.drag_icon
                && drag_icon.surface.wl_surface().id() == *surface_id
            {
                drag_icon.scale_changed(scale, &state.shm_state);
            }
        }
    }
}

impl ActivationHandler for ApplicationState {
    type RequestUdata = RequestId;

    fn new_token(&mut self, token: String, data: &RequestData<Self::RequestUdata>) {
        let request_id = data.udata;
        info!("ActivationHandler::new_token for {data:?}: {token}");
        self.send_event(ActivationTokenResponse::new(request_id, &token));
    }
}
