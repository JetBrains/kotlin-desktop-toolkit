use anyhow::Context;
use khronos_egl as egl;
use log::{debug, info, warn};
use smithay_client_toolkit::{
    compositor::SurfaceData,
    reexports::{
        client::{
            Connection, Proxy, QueueHandle,
            protocol::{wl_output::WlOutput, wl_seat::WlSeat, wl_shm, wl_surface::WlSurface},
        },
        csd_frame::{WindowManagerCapabilities, WindowState},
        protocols::{wp::viewporter::client::wp_viewport::WpViewport, xdg::shell::client::xdg_toplevel::ResizeEdge as XdgResizeEdge},
    },
    seat::pointer::{CursorIcon, ThemedPointer},
    shell::{
        WaylandSurface,
        xdg::{
            XdgSurface,
            window::{DecorationMode, Window, WindowConfigure, WindowDecorations},
        },
    },
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use wayland_egl::WlEglSurface;

use crate::linux::events::{WindowCapabilities, WindowResizeEvent};
use crate::linux::geometry::LogicalSize;
use crate::linux::{application_state::ApplicationState, geometry::LogicalPixels};

use super::application_state::EglInstance;
use super::events::{Event, InternalEventHandler, SoftwareDrawData, WindowDrawEvent, WindowFrameAction, WindowResizeEdge};
use super::geometry::PhysicalSize;
use super::window_api::WindowParams;

#[derive(Debug)]
struct EglRendering {
    wl_egl_surface: WlEglSurface,
    egl_display: egl::Display,
    egl_window_surface: khronos_egl::Surface,
}

impl EglRendering {
    fn new(conn: &Connection, egl: &EglInstance, surface: &WlSurface, size: PhysicalSize) -> anyhow::Result<Self> {
        info!("Trying to use EGL rendering");

        let wl_egl_surface = WlEglSurface::new(surface.id(), size.width.0, size.height.0).context("WlEglSurface::new")?;

        let wayland_display_ptr = conn.display().id().as_ptr();
        let egl_display = unsafe { egl.get_display(wayland_display_ptr.cast()) }.context("egl.get_display")?;
        egl.initialize(egl_display).context("egl.initialize")?;

        let egl_attributes = [
            egl::RED_SIZE,
            8,
            egl::GREEN_SIZE,
            8,
            egl::BLUE_SIZE,
            8,
            egl::ALPHA_SIZE,
            8,
            egl::NONE,
        ];

        let egl_config = egl
            .choose_first_config(egl_display, &egl_attributes)?
            .context("unable to find an appropriate ELG configuration")?;

        let egl_context_attributes = [egl::CONTEXT_MAJOR_VERSION, 3, egl::CONTEXT_MINOR_VERSION, 0, egl::NONE];

        let egl_context = egl
            .create_context(egl_display, egl_config, None, &egl_context_attributes)
            .context("egl.create_context")?;

        let egl_window_surface = unsafe { egl.create_window_surface(egl_display, egl_config, wl_egl_surface.ptr().cast_mut(), None) }
            .context("egl.create_window_surface")?;

        egl.make_current(egl_display, Some(egl_window_surface), Some(egl_window_surface), Some(egl_context))
            .context("egl.make_current")?;

        Ok(Self {
            wl_egl_surface,
            egl_display,
            egl_window_surface,
        })
    }

    fn draw<F: FnOnce(Option<SoftwareDrawData>)>(&self, surface: &WlSurface, egl: &EglInstance, do_draw: F) {
        do_draw(None);

        // Attach and commit to present.
        egl.swap_buffers(self.egl_display, self.egl_window_surface).unwrap();
        surface.commit();
    }
}

#[derive(Debug)]
struct SoftwareRendering {
    pool: SlotPool,
    buffer: Buffer,
    stride: i32,
}

struct SoftwareBuffer<'a> {
    buffer: Buffer,
    canvas: &'a mut [u8],
}

impl SoftwareRendering {
    const BYTES_PER_PIXEL: u8 = 4;

    fn create_buffer(pool: &mut SlotPool, size: PhysicalSize) -> SoftwareBuffer {
        let stride = size.width.0 * i32::from(Self::BYTES_PER_PIXEL);
        let (buffer, canvas) = pool
            .create_buffer(size.width.0, size.height.0, stride, wl_shm::Format::Argb8888)
            .expect("create buffer");
        SoftwareBuffer { buffer, canvas }
    }

    fn new(shm: &Shm, size: PhysicalSize) -> Self {
        let stride = size.width.0 * i32::from(Self::BYTES_PER_PIXEL);
        let mut pool = SlotPool::new(
            (stride * size.height.0 * 2).try_into().unwrap(), // double buffered
            shm,
        )
        .expect("Failed to create pool");
        let buffer = Self::create_buffer(&mut pool, size).buffer;
        Self { pool, buffer, stride }
    }

    fn resize(&mut self, shm: &Shm, size: PhysicalSize) {
        let stride = size.width.0 * i32::from(Self::BYTES_PER_PIXEL);
        if self.buffer.height() != size.height.0 || self.buffer.stride() != stride {
            *self = Self::new(shm, size);
        }
    }

    fn draw<F: FnOnce(Option<SoftwareDrawData>)>(&mut self, surface: &WlSurface, size: PhysicalSize, do_draw: F) {
        let canvas = if let Some(canvas) = self.pool.canvas(&self.buffer) {
            canvas
        } else {
            // This should be rare, but if the compositor has not released the previous
            // buffer, we need double-buffering.
            let second_draw_data = Self::create_buffer(&mut self.pool, size);
            self.buffer = second_draw_data.buffer;
            second_draw_data.canvas
        };

        do_draw(Some(SoftwareDrawData {
            canvas: canvas.as_mut_ptr(),
            stride: self.stride,
        }));

        // Attach and commit to present.
        self.buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
    }
}

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
    pub keyboard_focus: bool,
    pub set_cursor: bool,
    decorations_cursor: CursorIcon,
    current_scale: f64,
    decoration_mode: DecorationMode,
    rendering_data: Option<RenderingData>,
    force_software_rendering: bool,
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
            keyboard_focus: false,
            set_cursor: false,
            decorations_cursor: CursorIcon::Default,
            current_scale: 1.0,
            decoration_mode: DecorationMode::Client,
            rendering_data: None,
            force_software_rendering: params.force_software_rendering,
        }
    }

    pub fn request_close(&mut self) {
        (self.event_handler)(&Event::WindowCloseRequest);
        self.close = true;
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
                    egl_data.wl_egl_surface.resize(physical_size.width.0, physical_size.height.0, 0, 0);
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

    pub fn frame_action(&mut self, seat: &WlSeat, serial: u32, action: WindowFrameAction) {
        match action {
            WindowFrameAction::Close => self.close = true,
            WindowFrameAction::Minimize => self.window.set_minimized(),
            WindowFrameAction::Maximize => self.window.set_maximized(),
            WindowFrameAction::UnMaximize => self.window.unset_maximized(),
            WindowFrameAction::ShowMenu(x, y) => self.window.show_window_menu(seat, serial, (x, y)),
            WindowFrameAction::Resize(edge) => {
                let edge = match edge {
                    WindowResizeEdge::None => XdgResizeEdge::None,
                    WindowResizeEdge::Top => XdgResizeEdge::Top,
                    WindowResizeEdge::Bottom => XdgResizeEdge::Bottom,
                    WindowResizeEdge::Left => XdgResizeEdge::Left,
                    WindowResizeEdge::TopLeft => XdgResizeEdge::TopLeft,
                    WindowResizeEdge::BottomLeft => XdgResizeEdge::BottomLeft,
                    WindowResizeEdge::Right => XdgResizeEdge::Right,
                    WindowResizeEdge::TopRight => XdgResizeEdge::TopRight,
                    WindowResizeEdge::BottomRight => XdgResizeEdge::BottomRight,
                };
                self.window.resize(seat, serial, edge);
            }
            WindowFrameAction::Move => self.window.move_(seat, serial),
            WindowFrameAction::None => (),
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
}
