use std::num::NonZeroU32;
use std::sync::Arc;

use log::debug;
use smithay_client_toolkit::compositor::SurfaceData;
use smithay_client_toolkit::reexports::client::globals::GlobalList;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::reexports::csd_frame::{DecorationsFrame, FrameAction, ResizeEdge};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge as XdgResizeEdge;
use smithay_client_toolkit::shell::xdg::window::WindowDecorations;
use smithay_client_toolkit::{
    reexports::client::{
        Connection, Proxy, QueueHandle,
        protocol::{wl_pointer, wl_shm},
    },
    shm::Shm,
};
use smithay_client_toolkit::{
    seat::pointer::PointerData,
    shell::{
        WaylandSurface,
        xdg::{
            XdgSurface,
            fallback_frame::FallbackFrame,
            window::{DecorationMode, Window, WindowConfigure},
        },
    },
};

use crate::linux::application_state::ApplicationState;
use crate::linux::cursors::CURSORS;
use crate::linux::events::{LogicalPixels, LogicalSize};

use smithay_client_toolkit::{
    seat::pointer::{CursorIcon, ThemedPointer},
    shm::slot::{Buffer, SlotPool},
    subcompositor::SubcompositorState,
};

use super::events::{Event, InternalEventHandler};

#[repr(C)]
pub struct WindowParams {
    //pub origin: LogicalPoint,
    pub width: u32,
    pub height: u32,
    //pub title: BorrowedStrPtr<'a>,

    //pub is_resizable: bool,
    //pub is_closable: bool,
    //pub is_miniaturizable: bool,

    //pub is_full_screen_allowed: bool,
    //pub use_custom_titlebar: bool,
    //pub titlebar_height: LogicalPixels,
}

pub struct SimpleWindow {
    pub event_handler: Box<InternalEventHandler>,
    pub subcompositor_state: Arc<SubcompositorState>,
    pub close: bool,
    pub first_configure: bool,
    pub pool: SlotPool,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub buffer: Option<Buffer>,
    pub viewport: Option<WpViewport>,
    pub window: Window,
    pub window_frame: Option<FallbackFrame<ApplicationState>>,
    pub keyboard_focus: bool,
    pub set_cursor: bool,
    pub window_cursor_icon_idx: usize,
    pub decorations_cursor: Option<CursorIcon>,
    pub current_scale: f64,
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
        let window = state
            .xdg_shell_state
            .create_window(window_surface, WindowDecorations::ServerDefault, qh);
        window.set_title("A wayland window");
        // GitHub does not let projects use the `org.github` domain but the `io.github` domain is fine.
        window.set_app_id("io.github.smithay.client-toolkit.SimpleWindow");
        window.set_min_size(Some((width.get(), height.get())));

        // In order for the window to be mapped, we need to perform an initial commit with no attached buffer.
        // For more info, see WaylandSurface::commit
        //
        // The compositor will respond with an initial configure that we can then use to present to the window with
        // the correct options.
        window.commit();

        debug!("Created new window with surface_id={surface_id}");
        Self {
            event_handler,
            subcompositor_state: Arc::new(subcompositor_state),
            close: false,
            first_configure: true,
            pool,
            width,
            height,
            buffer: None,
            viewport,
            window,
            window_frame: None,
            keyboard_focus: false,
            set_cursor: false,
            window_cursor_icon_idx: 0,
            decorations_cursor: None,
            current_scale: 1.0,
        }
    }

    pub fn request_close(&mut self) {
        (self.event_handler)(&Event::WindowCloseRequest);
        self.close = true;
    }

    pub fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<ApplicationState>,
        shm: &Shm,
        window: &Window,
        configure: &WindowConfigure,
        themed_pointer: Option<&mut ThemedPointer>,
    ) {
        self.buffer = None;

        debug!(
            "Configure size {:?}, decorations: {:?}",
            configure.new_size, configure.decoration_mode
        );
        debug!("Supported formats: {:?}", shm.formats());
        // [Argb8888, Xrgb8888, Abgr8888, Xbgr8888, Rgb565, Argb2101010, Xrgb2101010, Abgr2101010, Xbgr2101010, Argb16161616f, Xrgb16161616f, Abgr16161616f, Xbgr16161616f, Yuyv, Nv12, P010, Yuv420]

        let (width, height) = if configure.decoration_mode == DecorationMode::Client {
            let window_frame = self.window_frame.get_or_insert_with(|| {
                FallbackFrame::new(&self.window, shm, self.subcompositor_state.clone(), qh.clone())
                    .expect("failed to create client side decorations frame.")
            });

            // Un-hide the frame.
            window_frame.set_hidden(false);

            // Configure state before touching any resizing.
            window_frame.update_state(configure.state);

            // Update the capabilities.
            window_frame.update_wm_capabilities(configure.capabilities);

            let (width, height) = match configure.new_size {
                (Some(width), Some(height)) => {
                    // The size could be 0.
                    window_frame.subtract_borders(width, height)
                }
                _ => {
                    // You might want to consider checking for configure bounds.
                    (Some(self.width), Some(self.height))
                }
            };

            // Clamp the size to at least one pixel.
            let width = width.unwrap_or(NonZeroU32::new(1).unwrap());
            let height = height.unwrap_or(NonZeroU32::new(1).unwrap());

            debug!("New dimentions: {width}, {height}");
            window_frame.resize(width, height);

            let (x, y) = window_frame.location();
            let outer_size = window_frame.add_borders(width.get(), height.get());
            window
                .xdg_surface()
                .set_window_geometry(x, y, outer_size.0 as i32, outer_size.1 as i32);

            (width, height)
        } else {
            // Hide the frame, if any.
            if let Some(frame) = self.window_frame.as_mut() {
                frame.set_hidden(true);
            }
            let width = configure.new_size.0.unwrap_or(self.width);
            let height = configure.new_size.1.unwrap_or(self.height);
            self.window
                .xdg_surface()
                .set_window_geometry(0, 0, width.get() as i32, height.get() as i32);
            (width, height)
        };

        // Update new width and height;
        self.width = width;
        self.height = height;

        if let Some(viewport) = &self.viewport {
            viewport.set_destination(self.width.get() as i32, self.height.get() as i32);
        }
        (self.event_handler)(&Event::new_window_resize_event(LogicalSize {
            width: LogicalPixels(width.get().into()),
            height: LogicalPixels(height.get().into()),
        }));

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(conn, qh, themed_pointer);
        }
    }

    pub fn frame_action(&mut self, pointer: &wl_pointer::WlPointer, serial: u32, action: FrameAction) {
        let pointer_data = pointer.data::<PointerData>().unwrap();
        let seat = pointer_data.seat();
        match action {
            FrameAction::Close => self.close = true,
            FrameAction::Minimize => self.window.set_minimized(),
            FrameAction::Maximize => self.window.set_maximized(),
            FrameAction::UnMaximize => self.window.unset_maximized(),
            FrameAction::ShowMenu(x, y) => self.window.show_window_menu(seat, serial, (x, y)),
            FrameAction::Resize(edge) => {
                let edge = match edge {
                    ResizeEdge::None => XdgResizeEdge::None,
                    ResizeEdge::Top => XdgResizeEdge::Top,
                    ResizeEdge::Bottom => XdgResizeEdge::Bottom,
                    ResizeEdge::Left => XdgResizeEdge::Left,
                    ResizeEdge::TopLeft => XdgResizeEdge::TopLeft,
                    ResizeEdge::BottomLeft => XdgResizeEdge::BottomLeft,
                    ResizeEdge::Right => XdgResizeEdge::Right,
                    ResizeEdge::TopRight => XdgResizeEdge::TopRight,
                    ResizeEdge::BottomRight => XdgResizeEdge::BottomRight,
                    _ => return,
                };
                self.window.resize(seat, serial, edge);
            }
            FrameAction::Move => self.window.move_(seat, serial),
            _ => (),
        }
    }

    pub fn draw(&mut self, conn: &Connection, qh: &QueueHandle<ApplicationState>, themed_pointer: Option<&mut ThemedPointer>) {
        let surface = self.window.wl_surface();
        if self.set_cursor {
            debug!("Updating cursor to {} for {}", self.window_cursor_icon_idx, surface.id());
            let cursor_icon = self.decorations_cursor.unwrap_or(CURSORS[self.window_cursor_icon_idx]);
            themed_pointer.unwrap().set_cursor(conn, cursor_icon).unwrap();
            self.set_cursor = false;
        }

        #[allow(clippy::cast_possible_truncation)]
        let width = (f64::from(self.width.get()) * self.current_scale).ceil() as i32;

        #[allow(clippy::cast_possible_truncation)]
        let height = (f64::from(self.height.get()) * self.current_scale).ceil() as i32;

        let stride = width * 4;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
                .expect("create buffer")
                .0
        });

        let canvas = if let Some(canvas) = self.pool.canvas(buffer) {
            canvas
        } else {
            // This should be rare, but if the compositor has not released the previous
            // buffer, we need double-buffering.
            let (second_buffer, canvas) = self
                .pool
                .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
                .expect("create buffer");
            *buffer = second_buffer;
            canvas
        };

        (self.event_handler)(&Event::new_window_draw_event(
            canvas,
            u32::try_from(width).unwrap(),
            u32::try_from(height).unwrap(),
            u32::try_from(stride).unwrap(),
            self.current_scale,
        ));

        // Draw the decorations frame.
        if let Some(frame) = self.window_frame.as_mut() {
            if frame.is_dirty() && !frame.is_hidden() {
                frame.draw();
            }
        }

        // Damage the entire window
        surface.damage_buffer(0, 0, width, height);

        if self.viewport.is_none() {
            assert!(self.current_scale % 1.0 == 0.0);
            #[allow(clippy::cast_possible_truncation)]
            surface.set_buffer_scale(self.current_scale as i32);
        }

        // Request our next frame
        surface.frame(qh, surface.clone());

        // Attach and commit to present.
        buffer.attach_to(surface).expect("buffer attach");
        surface.commit();
    }

    pub fn output_changed(&mut self, output: &WlOutput) {
        (self.event_handler)(&Event::new_window_screen_change_event(output));
    }

    pub fn scale_changed(&mut self, new_scale: f64) {
        self.current_scale = new_scale;
        (self.event_handler)(&Event::new_window_scale_changed_event(new_scale));
    }
}
