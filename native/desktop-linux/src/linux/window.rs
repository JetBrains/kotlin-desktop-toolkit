use std::num::NonZeroU32;
use std::sync::Arc;

use log::debug;
use smithay_client_toolkit::reexports::csd_frame::{DecorationsFrame, FrameAction, ResizeEdge};
use smithay_client_toolkit::reexports::protocols::xdg::shell::client::xdg_toplevel::ResizeEdge as XdgResizeEdge;
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

use smithay_client_toolkit::{
    seat::pointer::{CursorIcon, ThemedPointer},
    shm::slot::{Buffer, SlotPool},
    subcompositor::SubcompositorState,
};

pub struct SimpleWindow {
    pub subcompositor_state: Arc<SubcompositorState>,
    pub close: bool,
    pub first_configure: bool,
    pub pool: SlotPool,
    pub width: NonZeroU32,
    pub height: NonZeroU32,
    pub buffer: Option<Buffer>,
    pub window: Window,
    pub window_frame: Option<FallbackFrame<ApplicationState>>,
    pub keyboard_focus: bool,
    pub set_cursor: bool,
    pub window_cursor_icon_idx: usize,
    pub decorations_cursor: Option<CursorIcon>,
}

impl SimpleWindow {
    pub fn request_close(&mut self) {
        self.close = true;
    }

    pub fn configure(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<ApplicationState>,
        shm: &Shm,
        window: &Window,
        configure: WindowConfigure,
        themed_pointer: Option<&mut ThemedPointer>,
    ) {
        self.buffer = None;

        println!(
            "Configure size {:?}, decorations: {:?}",
            configure.new_size, configure.decoration_mode
        );

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

            println!("New dimentions: {width}, {height}");
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
                frame.set_hidden(true)
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
        if self.set_cursor {
            debug!(
                "Updating cursor to {} for {}",
                self.window_cursor_icon_idx,
                self.window.wl_surface().id()
            );
            let cursor_icon = self.decorations_cursor.unwrap_or(CURSORS[self.window_cursor_icon_idx]);
            themed_pointer.unwrap().set_cursor(conn, cursor_icon).unwrap();
            self.set_cursor = false;
        }

        let width = self.width.get();
        let height = self.height.get();
        let stride = self.width.get() as i32 * 4;

        let buffer = self.buffer.get_or_insert_with(|| {
            self.pool
                .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                .expect("create buffer")
                .0
        });

        let canvas = match self.pool.canvas(buffer) {
            Some(canvas) => canvas,
            None => {
                // This should be rare, but if the compositor has not released the previous
                // buffer, we need double-buffering.
                let (second_buffer, canvas) = self
                    .pool
                    .create_buffer(width as i32, height as i32, stride, wl_shm::Format::Argb8888)
                    .expect("create buffer");
                *buffer = second_buffer;
                canvas
            }
        };

        // Draw to the window:
        {
            for (i, pixel) in canvas.chunks_exact_mut(4).enumerate() {
                let i = i as u32;
                // Borders at 1px offset from sides
                if (i % width == 1)
                    || (i % width == (width - 2))
                    || ((i >= width) && (i < width * 2))
                    || ((i >= width * (height - 2)) && (i < width * (height - 1)))
                {
                    pixel[0] = 0;
                    pixel[1] = 0;
                    pixel[2] = 255;
                    pixel[3] = 255;
                } else {
                    pixel[0] = 255;
                    pixel[1] = 255;
                    pixel[2] = 255;
                    pixel[3] = 255;
                }
            }
        }

        // Draw the decorations frame.
        if let Some(frame) = self.window_frame.as_mut() {
            if frame.is_dirty() && !frame.is_hidden() {
                frame.draw();
            }
        }

        // Damage the entire window
        self.window.wl_surface().damage_buffer(0, 0, width as i32, height as i32);

        // Request our next frame
        self.window.wl_surface().frame(qh, self.window.wl_surface().clone());

        // Attach and commit to present.
        buffer.attach_to(self.window.wl_surface()).expect("buffer attach");
        self.window.wl_surface().commit();
    }
}
