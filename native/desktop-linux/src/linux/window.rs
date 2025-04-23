use log::{debug, info, warn};
use smithay_client_toolkit::{
    compositor::SurfaceData,
    reexports::{
        client::{
            Connection, Proxy, QueueHandle,
            protocol::{wl_output::WlOutput, wl_seat::WlSeat},
        },
        csd_frame::{WindowManagerCapabilities, WindowState},
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

use crate::linux::events::{WindowCapabilities, WindowResizeEvent};
use crate::linux::geometry::LogicalSize;
use crate::linux::{application_state::ApplicationState, geometry::LogicalPixels};

use super::{application_state::EglInstance, geometry::LogicalPoint};
use super::{
    events::{Event, InternalEventHandler, SoftwareDrawData, WindowDrawEvent},
    rendering_egl::EglRendering,
    rendering_software::SoftwareRendering,
};
use super::{window_api::WindowParams, window_resize_edge::WindowResizeEdge};

#[derive(Debug)]
enum RenderingData {
    Egl(EglRendering),
    Software(SoftwareRendering),
}

pub struct SimpleWindow {
    pub event_handler: Box<InternalEventHandler>,
    pub close: bool,
    pub size: Option<LogicalSize>,
    viewport: Option<WpViewport>,
    pub window: Window,
    pub set_cursor: bool,
    decorations_cursor: CursorIcon,
    current_scale: f64,
    decoration_mode: DecorationMode,
    rendering_data: Option<RenderingData>,
    force_software_rendering: bool,
    pub current_mouse_down_seat: Option<WlSeat>,
    pub current_mouse_down_serial: Option<u32>,
}

impl SimpleWindow {
    #[must_use]
    pub fn new(
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

        let d: Option<&SurfaceData> = window_surface.data();
        dbg!(d);

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

        debug!("Creating new window with surface_id={surface_id}");
        Self {
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
        debug!("Configure {configure:?}");

        self.decoration_mode = configure.decoration_mode;

        // debug!("Supported formats: {:?}", shm.formats());
        // [Argb8888, Xrgb8888, Abgr8888, Xbgr8888, Rgb565, Argb2101010, Xrgb2101010, Abgr2101010, Xbgr2101010, Argb16161616f, Xrgb16161616f, Abgr16161616f, Xbgr16161616f, Yuyv, Nv12, P010, Yuv420]

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

        if let Some(viewport) = &self.viewport {
            viewport.set_destination(width.round(), height.round());
        }

        (self.event_handler)(
            &WindowResizeEvent {
                size,
                maximized: configure.state.contains(WindowState::MAXIMIZED),
                fullscreen: configure.state.contains(WindowState::FULLSCREEN),
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

        if let Some(rendering_data) = &mut self.rendering_data {
            match rendering_data {
                RenderingData::Egl(egl_data) => {
                    egl_data.resize(physical_size);
                }
                RenderingData::Software(data) => {
                    data.resize(shm, physical_size);
                }
            }
            false
        } else {
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
            debug!("Updating cursor to {} for {}", self.decorations_cursor, surface.id());
            themed_pointer.unwrap().set_cursor(conn, self.decorations_cursor).unwrap();
            self.set_cursor = false;
        }

        let physical_size = self.size.unwrap().to_physical(self.current_scale);

        let do_draw = |software_draw_data: Option<SoftwareDrawData>| {
            (self.event_handler)(
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

            // Damage the entire window
            surface.damage_buffer(0, 0, physical_size.width.0, physical_size.height.0);

            if self.viewport.is_none() {
                assert!(self.current_scale % 1.0 == 0.0);
                #[allow(clippy::cast_possible_truncation)]
                surface.set_buffer_scale(self.current_scale as i32);
            }

            // Request our next frame
            surface.frame(qh, surface.clone());
        };

        match &mut self.rendering_data {
            Some(RenderingData::Egl(r)) => r.draw(surface, egl.unwrap(), do_draw),
            Some(RenderingData::Software(r)) => r.draw(surface, physical_size, do_draw),
            None => warn!("Rendering data not initialized in draw"),
        };
    }

    pub fn output_changed(&self, output: &WlOutput) {
        (self.event_handler)(&Event::new_window_screen_change_event(output));
    }

    pub fn scale_changed(&mut self, new_scale: f64) {
        debug!("scale_changed: {new_scale}");
        self.current_scale = new_scale;
        (self.event_handler)(&Event::new_window_scale_changed_event(new_scale));
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
