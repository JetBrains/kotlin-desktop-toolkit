use crate::linux::{
    application_api::RenderingMode,
    application_state::{ApplicationState, EGLData},
    events::{
        EventSerial, SoftwareDrawData, WindowCapabilities, WindowConfigureEvent, WindowDecorationMode, WindowDrawEvent, WindowFrame,
        WindowFrameTiling, WindowId,
    },
    geometry::{LogicalPixelsInt, LogicalPoint, LogicalRect, LogicalSize, PhysicalSize, Scale},
    pointer_shapes_api::PointerShape,
    rendering_egl::EglRendering,
    rendering_software::SoftwareRendering,
    window_api::WindowParams,
    window_resize_edge_api::WindowResizeEdge,
};
use log::{debug, error, warn};
use smithay_client_toolkit::{
    compositor::{CompositorState, FrameCallbackData, Region},
    reexports::{
        client::{
            Connection, Proxy as _, QueueHandle,
            protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
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
use std::rc::Rc;

pub enum RenderingData {
    Egl(EglRendering),
    Software(SoftwareRendering),
}

impl RenderingData {
    pub fn draw<F>(&mut self, surface: &WlSurface, size: PhysicalSize, do_draw: F)
    where
        F: FnOnce(SoftwareDrawData) -> bool,
    {
        match self {
            Self::Egl(r) => r.draw(surface, do_draw),
            Self::Software(r) => r.draw(surface, size, do_draw),
        }
    }
}

pub struct SimpleWindow {
    pub window_id: WindowId,
    pub app_id: String,
    pub close: bool,
    content_size: Option<LogicalSize>,
    surface_size: Option<LogicalSize>,
    last_geometry: Option<LogicalRect>,
    viewport: Option<WpViewport>,
    pub window: Window,
    pub set_cursor: bool,
    decorations_cursor: Option<CursorIcon>,
    pub current_scale: Scale,
    decoration_mode: DecorationMode,
    rendering_data: Option<RenderingData>,
    pub rendering_mode: RenderingMode,
    pub num_pointer_buttons_down: u32,
    default_client_side_decoration_frame: WindowFrame,
    last_configure_event: Option<WindowConfigureEvent>,
}

impl SimpleWindow {
    pub fn new(
        window_id: WindowId,
        app_state: &ApplicationState,
        qh: &QueueHandle<ApplicationState>,
        params: WindowParams,
    ) -> anyhow::Result<Self> {
        let state = app_state;

        let window_surface = state.compositor_state.create_surface(qh);

        let surface_id = window_surface.id();

        if let Some(fractional_scale_manager) = state.fractional_scale_manager.as_ref() {
            fractional_scale_manager.get_fractional_scale(&window_surface, qh, surface_id.clone());
        }

        let viewport = state.viewporter.as_ref().map(|vp| vp.get_viewport(&window_surface, qh, ()));

        let decorations = if params.prefer_client_side_decoration {
            WindowDecorations::RequestClient
        } else {
            WindowDecorations::ServerDefault
        };

        let window = state.xdg_shell_state.create_window(window_surface, decorations, qh);
        let app_id = params.app_id.get("WindowParams: app_id")?.to_owned();
        window.set_title(params.title.get("WindowParams title")?);
        window.set_app_id(app_id.clone());

        let mut size = params.size.validate();

        if let Some(min_size) = params.min_size.validate() {
            window
                .xdg_toplevel()
                .set_min_size(min_size.width.raw_logical(), min_size.height.raw_logical());
            if let Some(size) = &mut size {
                if size.width < min_size.width {
                    size.width = min_size.width;
                }
                if size.height < min_size.height {
                    size.height = min_size.height;
                }
            }
        }

        // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
        // For more info, see WaylandSurface::commit
        //
        // The compositor will respond with an initial `configure` that we can then use to present to the window with
        // the correct options.
        window.commit();

        debug!("Creating new window with id={:?} and surface_id={surface_id}", params.window_id);
        Ok(Self {
            window_id,
            app_id,
            close: false,
            content_size: size,
            surface_size: None,
            last_geometry: None,
            viewport,
            window,
            set_cursor: false,
            decorations_cursor: Some(CursorIcon::Default),
            current_scale: Scale::default(),
            decoration_mode: DecorationMode::Client,
            rendering_data: None,
            rendering_mode: params.rendering_mode,
            num_pointer_buttons_down: 0,
            default_client_side_decoration_frame: params.client_side_decoration_frame,
            last_configure_event: None,
        })
    }

    pub const fn close(&mut self) {
        self.close = true;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn configure(
        &mut self,
        shm: &Shm,
        compositor_state: &CompositorState,
        window: &Window,
        configure: &WindowConfigure,
        egl: Option<Rc<EGLData>>,
    ) -> (bool, Option<WindowConfigureEvent>) {
        const DEFAULT_WIDTH: LogicalPixelsInt = LogicalPixelsInt::new(640);
        const DEFAULT_HEIGHT: LogicalPixelsInt = LogicalPixelsInt::new(480);
        debug!("{:?}: configure start: {configure:?}", self.window_id);

        self.decoration_mode = configure.decoration_mode;

        let content_width = configure
            .new_size
            .0
            .map(|v| LogicalPixelsInt::new(v.get().cast_signed()))
            .or_else(|| self.content_size.map(|s| s.width))
            .or_else(|| configure.suggested_bounds.map(|(w, _h)| LogicalPixelsInt::new(w.cast_signed())))
            .unwrap_or(DEFAULT_WIDTH);

        let content_height = configure
            .new_size
            .1
            .map(|v| LogicalPixelsInt::new(v.get().cast_signed()))
            .or_else(|| self.content_size.map(|s| s.height))
            .or_else(|| configure.suggested_bounds.map(|(_w, h)| LogicalPixelsInt::new(h.cast_signed())))
            .unwrap_or(DEFAULT_HEIGHT);

        let content_size = LogicalSize {
            width: content_width,
            height: content_height,
        };
        self.content_size = Some(content_size);

        let frame = if configure.decoration_mode == DecorationMode::Server || configure.is_maximized() || configure.is_fullscreen() {
            WindowFrame::default()
        } else {
            self.default_client_side_decoration_frame.clone()
        };

        let surface_size = self.update_window_geometry(content_size, &frame, compositor_state, shm);

        // TODO: wl_surface::set_opaque_region?

        let physical_size = surface_size.to_rounded_physical(self.current_scale);
        debug!(
            "{:?}: configure: size={surface_size:?}, physical_size={physical_size:?}",
            self.window_id
        );

        self.on_resize(surface_size, physical_size, shm);

        let is_first_configure = self.rendering_data.is_none();
        if is_first_configure {
            self.rendering_data = if let Some(egl) = egl {
                match EglRendering::new(egl, window.wl_surface(), physical_size) {
                    Ok(egl_rendering_data) => Some(RenderingData::Egl(egl_rendering_data)),
                    Err(e) => {
                        warn!("Failed to create EGL rendering, falling back to software rendering. Error: {e:?}");
                        match SoftwareRendering::new(shm, physical_size) {
                            Ok(software_rendering_data) => Some(RenderingData::Software(software_rendering_data)),
                            Err(e) => {
                                warn!("Failed to create software rendering. Error: {e:?}");
                                None
                            }
                        }
                    }
                }
            } else {
                match SoftwareRendering::new(shm, physical_size) {
                    Ok(software_rendering_data) => Some(RenderingData::Software(software_rendering_data)),
                    Err(e) => {
                        warn!("Failed to create software rendering. Error: {e:?}");
                        None
                    }
                }
            };
        }

        let decoration_mode = match configure.decoration_mode {
            DecorationMode::Client => WindowDecorationMode::Client {
                frame,
                tiling: WindowFrameTiling {
                    left: configure.is_tiled_left(),
                    top: configure.is_tiled_top(),
                    right: configure.is_tiled_right(),
                    bottom: configure.is_tiled_bottom(),
                },
            },
            DecorationMode::Server => WindowDecorationMode::Server,
        };

        let event = WindowConfigureEvent {
            window_id: self.window_id,
            size: self.surface_size.unwrap(),
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

        let event = if self.last_configure_event.replace(event.clone()).is_none_or(|e| e != event) {
            Some(event)
        } else {
            None
        };

        (is_first_configure, event)
    }

    fn update_window_geometry(
        &mut self,
        content_size: LogicalSize,
        frame: &WindowFrame,
        compositor_state: &CompositorState,
        shm: &Shm,
    ) -> LogicalSize {
        let surface_size = LogicalSize {
            width: content_size.width + frame.padding.left + frame.padding.right,
            height: content_size.height + frame.padding.top + frame.padding.bottom,
        };
        if self.surface_size != Some(surface_size) {
            self.on_resize(surface_size, surface_size.to_rounded_physical(self.current_scale), shm);
            self.surface_size = Some(surface_size);
        }

        let geometry = LogicalRect {
            x: frame.padding.left,
            y: frame.padding.top,
            width: content_size.width,
            height: content_size.height,
        };

        if self.last_geometry != Some(geometry) {
            self.last_geometry = Some(geometry);

            self.window.xdg_surface().set_window_geometry(
                geometry.x.raw_logical(),
                geometry.y.raw_logical(),
                geometry.width.raw_logical(),
                geometry.height.raw_logical(),
            );

            let input_region_rect = LogicalRect {
                x: geometry.x - frame.resizer_thickness.left,
                y: geometry.y - frame.resizer_thickness.top,
                width: geometry.width + frame.resizer_thickness.left + frame.resizer_thickness.right,
                height: geometry.height + frame.resizer_thickness.top + frame.resizer_thickness.bottom,
            };

            if input_region_rect == geometry {
                self.window.set_input_region(None);
            } else {
                let input_region = Region::new(compositor_state).unwrap();
                input_region.add(
                    input_region_rect.x.raw_logical(),
                    input_region_rect.y.raw_logical(),
                    input_region_rect.width.raw_logical(),
                    input_region_rect.height.raw_logical(),
                );
                self.window.set_input_region(Some(input_region.wl_region()));
            }
        }

        surface_size
    }

    pub fn update_pointer(&mut self, conn: &Connection, themed_pointer: Option<&mut ThemedPointer>) {
        if self.set_cursor
            && let Some(themed_pointer) = themed_pointer
        {
            debug!("Updating cursor to {:?} for {:?}", self.decorations_cursor, self.window_id);
            if let Some(decorations_cursor) = self.decorations_cursor {
                match themed_pointer.set_cursor(conn, decorations_cursor) {
                    Ok(()) => {
                        self.set_cursor = false;
                    }
                    Err(e) => {
                        error!("Failed to set cursor, error: {e:?}");
                    }
                }
            } else if let Err(e) = themed_pointer.hide_cursor() {
                warn!("Failed to hide cursor: {e}");
            } else {
                self.set_cursor = false;
            }
        }
    }

    pub fn draw(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<ApplicationState>,
        themed_pointer: Option<&mut ThemedPointer>,
        callback: &dyn Fn(WindowDrawEvent) -> bool,
    ) {
        self.update_pointer(conn, themed_pointer);
        let surface = self.window.wl_surface();

        let physical_size = self.surface_size.unwrap().to_rounded_physical(self.current_scale);

        let do_draw = |software_draw_data: SoftwareDrawData| {
            let did_draw = callback(WindowDrawEvent {
                window_id: self.window_id,
                software_draw_data,
                physical_size,
            });

            if did_draw {
                // Damage the entire window
                surface.damage_buffer(0, 0, physical_size.width.raw_physical(), physical_size.height.raw_physical());
            }

            // Request our next frame
            surface.frame(qh, FrameCallbackData(surface.clone()));
            did_draw
        };

        if let Some(r) = &mut self.rendering_data {
            r.draw(surface, physical_size, do_draw);
        } else {
            warn!("Rendering data not initialized in draw");
        }

        surface.commit();
    }

    fn on_resize(&mut self, size: LogicalSize, physical_size: PhysicalSize, shm: &Shm) {
        if let Some(viewport) = &self.viewport {
            debug!(
                "{:?}: viewport.set_destination({:?}, {:?}) for {:?}",
                self.window_id, size.width, size.height, self.window_id
            );
            viewport.set_destination(size.width.raw_logical(), size.height.raw_logical());
        } else {
            let surface = self.window.wl_surface();
            let buffer_scale = self.current_scale.to_scale_factor();
            debug!("{:?}: surface.set_buffer_scale({buffer_scale})", self.window_id);
            surface.set_buffer_scale(buffer_scale);
        }

        if let Some(rendering_data) = &mut self.rendering_data {
            match rendering_data {
                RenderingData::Egl(egl_data) => {
                    egl_data.resize(physical_size);
                }
                RenderingData::Software(data) => {
                    if let Err(e) = data.resize(shm, physical_size) {
                        warn!("Error resizing software renderer for window: {e}");
                    }
                }
            }
        }
    }

    pub fn scale_changed(&mut self, new_scale: Scale, shm: &Shm) {
        debug!("{:?}: scale_changed: {new_scale:?}", self.window_id);
        self.current_scale = new_scale;

        if let Some(size) = self.surface_size {
            self.on_resize(size, size.to_rounded_physical(self.current_scale), shm);
        }
    }

    pub fn set_cursor_icon(&mut self, pointer_shape: PointerShape) {
        let cursor_icon = pointer_shape.to_wayland_cursor_icon();
        if self.decorations_cursor != cursor_icon {
            self.set_cursor = true;
            self.decorations_cursor = cursor_icon;
        }
    }

    pub fn start_move(&self, seat: &WlSeat, event_serial: EventSerial) {
        self.window.move_(seat, event_serial.0);
    }

    pub fn start_resize(&self, edge: WindowResizeEdge, seat: &WlSeat, event_serial: EventSerial) {
        self.window.resize(seat, event_serial.0, edge.into());
    }

    pub fn show_menu(&self, position: LogicalPoint, seat: &WlSeat, event_serial: EventSerial) {
        self.window
            .show_window_menu(seat, event_serial.0, (position.x.round(), position.y.round()));
    }

    pub fn set_max_size(&self, max_size: LogicalSize) {
        self.window
            .xdg_toplevel()
            .set_max_size(max_size.width.raw_logical(), max_size.height.raw_logical());
        self.window.commit();
    }

    pub fn set_min_size(&self, min_size: LogicalSize) {
        self.window
            .xdg_toplevel()
            .set_min_size(min_size.width.raw_logical(), min_size.height.raw_logical());
        self.window.commit();
    }

    #[must_use]
    pub fn set_client_side_decoration_frame(
        &mut self,
        frame: WindowFrame,
        compositor_state: &CompositorState,
        shm: &Shm,
    ) -> Option<WindowConfigureEvent> {
        debug!("{:?}: set_client_side_decoration_frame: {frame:?}", self.window_id);

        if self.default_client_side_decoration_frame == frame {
            return None;
        }

        self.default_client_side_decoration_frame = frame.clone();

        if let Some(mut event) = self.last_configure_event.clone()
            && let WindowDecorationMode::Client {
                frame: last_configure_event_frame,
                ..
            } = &mut event.decoration_mode
            && !event.maximized
            && !event.fullscreen
        {
            if let Some(content_size) = self.content_size {
                event.size = self.update_window_geometry(content_size, &frame, compositor_state, shm);
            }
            *last_configure_event_frame = frame;

            self.last_configure_event = Some(event.clone());
            Some(event)
        } else {
            None
        }
    }

    pub fn set_prefer_client_side_decoration(&self, prefer_client_side_decoration: bool) {
        debug!(
            "{:?}: set_prefer_client_side_decoration: {prefer_client_side_decoration}",
            self.window_id
        );
        if prefer_client_side_decoration {
            self.window.request_decoration_mode(Some(DecorationMode::Client));
        } else {
            self.window.request_decoration_mode(Some(DecorationMode::Server));
        }
    }
}
