use std::num::NonZeroU32;

use desktop_common::ffi_utils::BorrowedStrPtr;
use khronos_egl as egl;
use log::{debug, info, warn};
use smithay_client_toolkit::compositor::SurfaceData;
use smithay_client_toolkit::reexports::client::globals::GlobalList;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::client::protocol::wl_seat::WlSeat;
use smithay_client_toolkit::reexports::client::protocol::wl_shm;
use smithay_client_toolkit::reexports::client::{Connection, Proxy, QueueHandle};
use smithay_client_toolkit::reexports::csd_frame::{WindowManagerCapabilities, WindowState};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge as XdgResizeEdge;
use smithay_client_toolkit::shell::xdg::window::{DecorationMode, WindowDecorations};
use smithay_client_toolkit::shell::{
    WaylandSurface,
    xdg::{
        XdgSurface,
        window::{Window, WindowConfigure},
    },
};
use wayland_egl::WlEglSurface;

use crate::linux::application_state::ApplicationState;
use crate::linux::events::{LogicalPixels, LogicalSize, WindowCapabilities, WindowResizeEvent};

use smithay_client_toolkit::{
    seat::pointer::{CursorIcon, ThemedPointer},
    shm::slot::{Buffer, SlotPool},
    subcompositor::SubcompositorState,
};

use super::application_state::EglInstance;
use super::events::{Event, InternalEventHandler, WindowDrawEvent};
use super::pointer_shapes::PointerShape;

#[repr(C)]
pub struct WindowParams<'a> {
    pub width: u32,

    pub height: u32,

    pub title: BorrowedStrPtr<'a>,

    /// See <https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:set_app_id>
    pub app_id: BorrowedStrPtr<'a>,

    pub force_client_side_decoration: bool,

    pub force_software_rendering: bool,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum WindowResizeEdge {
    /// Nothing is being dragged.
    None,
    /// The top edge is being dragged.
    Top,
    /// The bottom edge is being dragged.
    Bottom,
    /// The left edge is being dragged.
    Left,
    /// The top left corner is being dragged.
    TopLeft,
    /// The bottom left corner is being dragged.
    BottomLeft,
    /// The right edge is being dragged.
    Right,
    /// The top right corner is being dragged.
    TopRight,
    /// The bottom right corner is being dragged.
    BottomRight,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub enum WindowFrameAction {
    None,
    /// The window should be minimized.
    Minimize,
    /// The window should be maximized.
    Maximize,
    /// The window should be unmaximized.
    UnMaximize,
    /// The window should be closed.
    Close,
    /// An interactive move should be started.
    Move,
    /// An interactive resize should be started with the provided edge.
    Resize(WindowResizeEdge),
    /// Show window menu.
    ///
    /// The coordinates are relative to the base surface, as in should be
    /// directly passed to the `xdg_toplevel::show_window_menu`.
    ShowMenu(i32, i32),
}

pub struct EglData {
    pub egl_surface: WlEglSurface,
    pub egl_display: egl::Display,
    pub egl_window_surface: khronos_egl::Surface,
}

pub struct SimpleWindow {
    pub event_handler: Box<InternalEventHandler>,
    pub subcompositor_state: SubcompositorState,
    pub close: bool,
    pub first_configure: bool,
    pub pool: SlotPool,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub buffer: Option<Buffer>,
    pub viewport: Option<WpViewport>,
    pub window: Window,
    pub keyboard_focus: bool,
    pub set_cursor: bool,
    pub decorations_cursor: CursorIcon,
    pub current_scale: f64,
    pub decoration_mode: DecorationMode,
    pub force_software_rendering: bool,
    pub egl_data: Option<EglData>,
}

impl SimpleWindow {
    #[must_use]
    pub fn new(
        app_state: &ApplicationState,
        globals: &GlobalList,
        qh: &QueueHandle<ApplicationState>,
        event_handler: Box<InternalEventHandler>,
        params: &WindowParams,
    ) -> Self {
        let state = app_state;
        let width = NonZeroU32::new(params.width).unwrap();
        let height = NonZeroU32::new(params.height).unwrap();
        let pool = SlotPool::new(width.get() as usize * height.get() as usize * 4, &state.shm_state).expect("Failed to create pool");

        let subcompositor_state =
            SubcompositorState::bind(state.compositor_state.wl_compositor().clone(), globals, qh).expect("wl_subcompositor not available");
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
        //window.set_min_size(Some((width.get(), height.get())));

        // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
        // For more info, see WaylandSurface::commit
        //
        // The compositor will respond with an initial configure that we can then use to present to the window with
        // the correct options.
        window.commit();

        debug!("Creating new window with surface_id={surface_id}");
        Self {
            event_handler,
            subcompositor_state,
            close: false,
            first_configure: true,
            pool,
            width,
            height,
            buffer: None,
            viewport,
            window,
            keyboard_focus: false,
            set_cursor: false,
            decorations_cursor: CursorIcon::Default,
            current_scale: 1.0,
            decoration_mode: DecorationMode::Client,
            egl_data: None,
            force_software_rendering: params.force_software_rendering,
        }
    }

    pub fn request_close(&mut self) {
        (self.event_handler)(&Event::WindowCloseRequest);
        self.close = true;
    }

    fn get_physical_size(&self) -> (i32, i32) {
        #[allow(clippy::cast_possible_truncation)]
        let physical_width = (f64::from(self.width.get()) * self.current_scale).ceil() as i32;

        #[allow(clippy::cast_possible_truncation)]
        let physical_height = (f64::from(self.height.get()) * self.current_scale).ceil() as i32;

        (physical_width, physical_height)
    }

    pub(crate) fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<ApplicationState>,
        window: &Window,
        configure: &WindowConfigure,
        themed_pointer: Option<&mut ThemedPointer>,
        egl: Option<&EglInstance>,
    ) {
        debug!("Configure {configure:?}");

        self.buffer = None;
        self.decoration_mode = configure.decoration_mode;

        // debug!("Supported formats: {:?}", shm.formats());
        // [Argb8888, Xrgb8888, Abgr8888, Xbgr8888, Rgb565, Argb2101010, Xrgb2101010, Abgr2101010, Xbgr2101010, Argb16161616f, Xrgb16161616f, Abgr16161616f, Xbgr16161616f, Yuyv, Nv12, P010, Yuv420]

        let width = configure.new_size.0.unwrap_or(self.width);
        let height = configure.new_size.1.unwrap_or(self.height);
        window
            .xdg_surface()
            .set_window_geometry(0, 0, width.get() as i32, height.get() as i32);

        // Update new width and height;
        self.width = width;
        self.height = height;

        let (physical_width, physical_height) = self.get_physical_size();

        if self.first_configure {
            if self.force_software_rendering {
                info!("Forcing software rendering");
            } else if let Some(egl) = egl {
                if let Ok(egl_surface) = WlEglSurface::new(window.wl_surface().id(), physical_width, physical_height) {
                    info!("Using EGL rendering");

                    let wayland_display_ptr = conn.display().id().as_ptr();
                    let egl_display = unsafe { egl.get_display(wayland_display_ptr.cast()).unwrap() };
                    egl.initialize(egl_display).unwrap();

                    let egl_attributes = [egl::RED_SIZE, 8, egl::GREEN_SIZE, 8, egl::BLUE_SIZE, 8, egl::NONE];

                    let egl_config = egl
                        .choose_first_config(egl_display, &egl_attributes)
                        .expect("unable to find an appropriate ELG configuration")
                        .unwrap();

                    let egl_context_attributes = [egl::CONTEXT_MAJOR_VERSION, 3, egl::CONTEXT_MINOR_VERSION, 0, egl::NONE];

                    let egl_context = egl.create_context(egl_display, egl_config, None, &egl_context_attributes).unwrap();

                    let egl_window_surface = unsafe {
                        egl.create_window_surface(egl_display, egl_config, egl_surface.ptr().cast_mut(), None)
                            .unwrap()
                    };
                    egl.make_current(egl_display, Some(egl_window_surface), Some(egl_window_surface), Some(egl_context))
                        .unwrap();

                    self.egl_data = Some(EglData {
                        egl_surface,
                        egl_display,
                        egl_window_surface,
                    });
                } else {
                    warn!("Failed to create EGL rendering, falling back to software rendering");
                };
            } else {
                warn!("Couldn't load EGL library, falling back to software rendering");
            }
        } else if let Some(egl_data) = &mut self.egl_data {
            egl_data.egl_surface.resize(physical_width, physical_height, 0, 0);
        }

        if let Some(viewport) = &self.viewport {
            viewport.set_destination(self.width.get() as i32, self.height.get() as i32);
        }

        (self.event_handler)(
            &WindowResizeEvent {
                size: LogicalSize {
                    width: LogicalPixels(width.get().into()),
                    height: LogicalPixels(height.get().into()),
                },
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

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh, themed_pointer, egl);
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

        let (physical_width, physical_height) = self.get_physical_size();
        let stride = physical_width * 4;

        let canvas = {
            if self.egl_data.is_some() {
                None
            } else {
                let buffer = self.buffer.get_or_insert_with(|| {
                    self.pool
                        .create_buffer(physical_width, physical_height, stride, wl_shm::Format::Argb8888)
                        .expect("create buffer")
                        .0
                });

                if let Some(canvas) = self.pool.canvas(buffer) {
                    Some(canvas)
                } else {
                    // This should be rare, but if the compositor has not released the previous
                    // buffer, we need double-buffering.
                    let (second_buffer, canvas) = self
                        .pool
                        .create_buffer(physical_width, physical_height, stride, wl_shm::Format::Argb8888)
                        .expect("create buffer");
                    *buffer = second_buffer;
                    Some(canvas)
                }
            }
        };
        (self.event_handler)(
            &WindowDrawEvent {
                buffer: canvas.map_or_else(std::ptr::null_mut, <[u8]>::as_mut_ptr),
                physical_width,
                physical_height,
                stride,
                scale: self.current_scale,
            }
            .into(),
        );

        // Damage the entire window
        surface.damage_buffer(0, 0, physical_width, physical_height);

        if self.viewport.is_none() {
            assert!(self.current_scale % 1.0 == 0.0);
            #[allow(clippy::cast_possible_truncation)]
            surface.set_buffer_scale(self.current_scale as i32);
        }

        // Request our next frame
        surface.frame(qh, surface.clone());

        // Attach and commit to present.
        match (egl, &self.egl_data) {
            (Some(egl), Some(egl_data)) => {
                egl.swap_buffers(egl_data.egl_display, egl_data.egl_window_surface).unwrap();
            }
            _ => {
                if let Some(buffer) = &self.buffer {
                    buffer.attach_to(surface).expect("buffer attach");
                }
            }
        }
        surface.commit();
    }

    pub fn output_changed(&mut self, output: &WlOutput) {
        (self.event_handler)(&Event::new_window_screen_change_event(output));
    }

    pub fn scale_changed(&mut self, new_scale: f64) {
        debug!("scale_changed: {new_scale}");
        self.current_scale = new_scale;
        (self.event_handler)(&Event::new_window_scale_changed_event(new_scale));
    }

    pub fn set_pointer_shape(&mut self, pointer_shape: PointerShape) {
        self.set_cursor = true;
        self.decorations_cursor = match pointer_shape {
            PointerShape::Default => CursorIcon::Default,
            PointerShape::ContextMenu => CursorIcon::ContextMenu,
            PointerShape::Help => CursorIcon::Help,
            PointerShape::Pointer => CursorIcon::Pointer,
            PointerShape::Progress => CursorIcon::Progress,
            PointerShape::Wait => CursorIcon::Wait,
            PointerShape::Cell => CursorIcon::Cell,
            PointerShape::Crosshair => CursorIcon::Crosshair,
            PointerShape::Text => CursorIcon::Text,
            PointerShape::VerticalText => CursorIcon::VerticalText,
            PointerShape::Alias => CursorIcon::Alias,
            PointerShape::Copy => CursorIcon::Copy,
            PointerShape::Move => CursorIcon::Move,
            PointerShape::NoDrop => CursorIcon::NoDrop,
            PointerShape::NotAllowed => CursorIcon::NotAllowed,
            PointerShape::Grab => CursorIcon::Grab,
            PointerShape::Grabbing => CursorIcon::Grabbing,
            PointerShape::EResize => CursorIcon::EResize,
            PointerShape::NResize => CursorIcon::NResize,
            PointerShape::NeResize => CursorIcon::NeResize,
            PointerShape::NwResize => CursorIcon::NwResize,
            PointerShape::SResize => CursorIcon::SResize,
            PointerShape::SeResize => CursorIcon::SeResize,
            PointerShape::SwResize => CursorIcon::SwResize,
            PointerShape::WResize => CursorIcon::WResize,
            PointerShape::EwResize => CursorIcon::EwResize,
            PointerShape::NsResize => CursorIcon::NsResize,
            PointerShape::NeswResize => CursorIcon::NeswResize,
            PointerShape::NwseResize => CursorIcon::NwseResize,
            PointerShape::ColResize => CursorIcon::ColResize,
            PointerShape::RowResize => CursorIcon::RowResize,
            PointerShape::AllScroll => CursorIcon::AllScroll,
            PointerShape::ZoomIn => CursorIcon::ZoomIn,
            PointerShape::ZoomOut => CursorIcon::ZoomOut,
        };
    }
}
