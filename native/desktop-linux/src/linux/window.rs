use log::{debug, error, info, warn};
use smithay_client_toolkit::{
    reexports::{
        client::{
            Connection, Proxy, QueueHandle,
            protocol::{wl_output::WlOutput, wl_seat::WlSeat},
        },
        csd_frame::WindowManagerCapabilities,
        protocols::wp::viewporter::client::wp_viewport::WpViewport,
    },
    seat::pointer::{CursorIcon, ThemedPointer},
    shell::{
        WaylandSurface,
        xdg::{
            XdgSurface,
            window::{DecorationMode, Window, WindowConfigure, WindowDecorations},
        },
    },
    shm::Shm,
};

use crate::linux::{
    application_state::{ApplicationState, EglInstance},
    events::{
        Event, InternalEventHandler, SoftwareDrawData, WindowCapabilities, WindowConfigureEvent, WindowDrawEvent, WindowId,
        WindowScaleChangedEvent, WindowScreenChangeEvent,
    },
    geometry::{LogicalPixels, LogicalPoint, LogicalSize, PhysicalSize},
    rendering_egl::EglRendering,
    rendering_software::SoftwareRendering,
    window_api::WindowParams,
    window_resize_edge::WindowResizeEdge,
};

#[derive(Debug)]
enum RenderingData {
    Egl(EglRendering),
    Software(SoftwareRendering),
}

pub struct SimpleWindow {
    pub window_id: WindowId,
    pub event_handler: Box<InternalEventHandler>,
    pub close: bool,
    pub size: Option<LogicalSize>,
    viewport: Option<WpViewport>,
    pub window: Window,
    pub set_cursor: bool,
    decorations_cursor: CursorIcon,
    pub current_scale: f64,
    decoration_mode: DecorationMode,
    rendering_data: Option<RenderingData>,
    force_software_rendering: bool,
    pub current_mouse_down_seat: Option<WlSeat>,
    pub current_mouse_down_serial: Option<u32>,
}

impl SimpleWindow {
    #[must_use]
    pub fn new(
        window_id: WindowId,
        app_state: &ApplicationState,
        qh: &QueueHandle<ApplicationState>,
        event_handler: Box<InternalEventHandler>,
        params: &WindowParams,
    ) -> Self {
        let state = app_state;

        let window_surface = state.compositor_state.create_surface(qh);

        let surface_id = window_surface.id();

        if let Some(fractional_scale_manager) = state.fractional_scale_manager.as_ref() {
            fractional_scale_manager.get_fractional_scale(&window_surface, qh, surface_id.clone());
        }

        let viewport = state.viewporter.as_ref().map(|vp| vp.get_viewport(&window_surface, qh, ()));

        let decorations = if params.force_client_side_decoration {
            WindowDecorations::RequestClient
        } else {
            WindowDecorations::ServerDefault
        };
        let window = state.xdg_shell_state.create_window(window_surface, decorations, qh);
        window.set_title(params.title.as_str().unwrap());
        window.set_app_id(params.app_id.as_str().unwrap());

        let size = if params.size.width.0 == 0.0 { None } else { Some(params.size) };

        // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
        // For more info, see WaylandSurface::commit
        //
        // The compositor will respond with an initial configure that we can then use to present to the window with
        // the correct options.
        window.commit();

        debug!("Creating new window with id={:?} and surface_id={surface_id}", params.window_id);
        Self {
            window_id,
            event_handler,
            close: false,
            size,
            viewport,
            window,
            set_cursor: false,
            decorations_cursor: CursorIcon::Default,
            current_scale: 1.0,
            decoration_mode: DecorationMode::Client,
            rendering_data: None,
            force_software_rendering: params.force_software_rendering,
            current_mouse_down_seat: None,
            current_mouse_down_serial: None,
        }
    }

    pub fn request_close(&self) {
        (self.event_handler)(&Event::WindowCloseRequest);
    }

    pub fn close(&mut self) {
        self.close = true;
        self.event_handler = Box::new(|_| false);
    }

    pub fn configure(
        &mut self,
        conn: &Connection,
        shm: &Shm,
        window: &Window,
        configure: &WindowConfigure,
        egl: Option<&EglInstance>,
    ) -> bool {
        const DEFAULT_WIDTH: LogicalPixels = LogicalPixels(640.);
        const DEFAULT_HEIGHT: LogicalPixels = LogicalPixels(480.);
        debug!("SimpleWindow::configure start: {configure:?}");

        self.decoration_mode = configure.decoration_mode;

        let width = configure
            .new_size
            .0
            .map(|w| LogicalPixels(w.get().into()))
            .or_else(|| self.size.map(|s| s.width))
            .or_else(|| configure.suggested_bounds.map(|(w, _h)| LogicalPixels(w.into())))
            .unwrap_or(DEFAULT_WIDTH);
        let height = configure
            .new_size
            .1
            .map(|h| LogicalPixels(h.get().into()))
            .or_else(|| self.size.map(|s| s.height))
            .or_else(|| configure.suggested_bounds.map(|(_w, h)| LogicalPixels(h.into())))
            .unwrap_or(DEFAULT_HEIGHT);
        let size = LogicalSize { width, height };
        self.size = Some(size);

        window.xdg_surface().set_window_geometry(0, 0, width.round(), height.round());
        // TODO: wl_surface::set_opaque_region?

        (self.event_handler)(
            &WindowConfigureEvent {
                size,
                active: configure.is_activated(),
                maximized: configure.is_maximized(),
                fullscreen: configure.is_fullscreen(),
                client_side_decorations: configure.decoration_mode == DecorationMode::Client,
                capabilities: WindowCapabilities {
                    window_menu: configure.capabilities.contains(WindowManagerCapabilities::WINDOW_MENU),
                    maximixe: configure.capabilities.contains(WindowManagerCapabilities::MAXIMIZE),
                    fullscreen: configure.capabilities.contains(WindowManagerCapabilities::FULLSCREEN),
                    minimize: configure.capabilities.contains(WindowManagerCapabilities::MINIMIZE),
                },
            }
            .into(),
        );

        let physical_size = size.to_physical(self.current_scale);
        debug!("SimpleWindow::configure: size={size:?}, physical_size={physical_size:?}");

        self.on_resize(&size, physical_size, shm);

        if self.rendering_data.is_none() {
            self.rendering_data = if self.force_software_rendering {
                info!("Forcing software rendering");
                Some(RenderingData::Software(SoftwareRendering::new(shm, physical_size)))
            } else if let Some(egl) = egl {
                match EglRendering::new(conn, egl, window.wl_surface(), physical_size) {
                    Ok(egl_rendering_data) => Some(RenderingData::Egl(egl_rendering_data)),
                    Err(e) => {
                        warn!("Failed to create EGL rendering, falling back to software rendering. Error: {e:?}");
                        Some(RenderingData::Software(SoftwareRendering::new(shm, physical_size)))
                    }
                }
            } else {
                warn!("Couldn't load EGL library, falling back to software rendering");
                Some(RenderingData::Software(SoftwareRendering::new(shm, physical_size)))
            };
            true
        } else {
            false
        }
    }

    pub fn draw(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<ApplicationState>,
        themed_pointer: Option<&mut ThemedPointer>,
        egl: Option<&EglInstance>,
    ) {
        let surface = self.window.wl_surface();
        if self.set_cursor {
            if let Some(themed_pointer) = themed_pointer {
                debug!("Updating cursor to {} for {}", self.decorations_cursor, surface.id());
                match themed_pointer.set_cursor(conn, self.decorations_cursor) {
                    Ok(()) => {
                        self.set_cursor = false;
                    }
                    Err(e) => {
                        error!("Failed to set cursor, error: {e:?}");
                    }
                }
            }
        }

        let physical_size = self.size.unwrap().to_physical(self.current_scale);

        let do_draw = |software_draw_data: Option<SoftwareDrawData>| {
            let did_draw = (self.event_handler)(
                &WindowDrawEvent {
                    software_draw_data: software_draw_data.unwrap_or(SoftwareDrawData {
                        canvas: std::ptr::null_mut(),
                        stride: 0,
                    }),
                    physical_size,
                    scale: self.current_scale,
                }
                .into(),
            );

            if did_draw {
                // Damage the entire window
                surface.damage_buffer(0, 0, physical_size.width.0, physical_size.height.0);
            }

            // Request our next frame
            surface.frame(qh, surface.clone());
            did_draw
        };

        match &mut self.rendering_data {
            Some(RenderingData::Egl(r)) => r.draw(surface, egl.unwrap(), do_draw),
            Some(RenderingData::Software(r)) => r.draw(surface, physical_size, do_draw),
            None => warn!("Rendering data not initialized in draw"),
        };

        surface.commit();
    }

    pub fn output_changed(&self, output: &WlOutput) {
        (self.event_handler)(&WindowScreenChangeEvent::new(output).into());
    }

    fn on_resize(&mut self, size: &LogicalSize, physical_size: PhysicalSize, shm: &Shm) {
        if let Some(viewport) = &self.viewport {
            debug!("viewport.set_destination({}, {})", size.width.round(), size.height.round());
            viewport.set_destination(size.width.round(), size.height.round());
        } else {
            let surface = self.window.wl_surface();
            assert!(self.current_scale % 1.0 == 0.0);
            debug!("surface.set_buffer_scale({})", self.current_scale);
            #[allow(clippy::cast_possible_truncation)]
            surface.set_buffer_scale(self.current_scale as i32);
        }

        if let Some(rendering_data) = &mut self.rendering_data {
            match rendering_data {
                RenderingData::Egl(egl_data) => {
                    egl_data.resize(physical_size);
                }
                RenderingData::Software(data) => {
                    data.resize(shm, physical_size);
                }
            }
        }
    }

    pub fn scale_changed(&mut self, new_scale: f64, shm: &Shm) {
        debug!("scale_changed: {new_scale}");
        self.current_scale = new_scale;

        if let Some(size) = self.size {
            self.on_resize(&size, size.to_physical(self.current_scale), shm);
        }

        (self.event_handler)(&WindowScaleChangedEvent { new_scale }.into());
    }

    pub fn set_cursor_icon(&mut self, cursor_icon: CursorIcon) {
        self.set_cursor = true;
        self.decorations_cursor = cursor_icon;
    }

    pub fn start_move(&self) {
        let serial = self.current_mouse_down_serial.unwrap();
        let seat = self.current_mouse_down_seat.as_ref().unwrap();
        self.window.move_(seat, serial);
    }

    pub fn start_resize(&self, edge: WindowResizeEdge) {
        let serial = self.current_mouse_down_serial.unwrap();
        let seat = self.current_mouse_down_seat.as_ref().unwrap();
        self.window.resize(seat, serial, edge.into());
    }

    pub fn show_menu(&self, position: LogicalPoint) {
        let serial = self.current_mouse_down_serial.unwrap();
        let seat = self.current_mouse_down_seat.as_ref().unwrap();
        self.window.show_window_menu(seat, serial, (position.x.round(), position.y.round()));
    }
}
