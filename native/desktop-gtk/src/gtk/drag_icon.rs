use anyhow::bail;
use gdk4::prelude::{Cast, DragExt, DragSurfaceExt, SurfaceExt};

use crate::gtk::application::send_event;
use crate::gtk::events::{Event, EventHandler};
use crate::gtk::rendering_gl::GlRendering;
use crate::gtk::{events::DragIconDrawEvent, geometry::LogicalSize};
use gtk4::gdk as gdk4;

pub struct DragIcon {
    pub surface: gdk4::DragSurface,
}

impl DragIcon {
    pub fn new(event_handler: EventHandler, drag: &gdk4::Drag, size: LogicalSize) -> anyhow::Result<Self> {
        let Some(drag_surface) = drag.drag_surface() else {
            bail!("Drag has no surface");
        };

        let rendering_data = GlRendering::new(&drag_surface).expect("Failed to create rendering data");
        drag_surface.connect_render(move |surface, _region| {
            let size = LogicalSize {
                width: surface.width(),
                height: surface.height(),
            };
            let scale = f64::from(surface.scale_factor());
            let physical_size = size.to_physical(scale);
            rendering_data.draw(size, |opengl_draw_data| {
                send_event(
                    event_handler,
                    DragIconDrawEvent {
                        opengl_draw_data,
                        physical_size,
                        scale,
                    },
                );
            });
            true
        });
        let drag_surface: gdk4::DragSurface = drag_surface.downcast().expect("Cannot cast surface to DragSurface");
        drag_surface.present(size.width, size.height);
        let frame_clock = drag_surface.frame_clock();
        frame_clock.connect_update(move |_| {
            send_event(event_handler, Event::ShouldRedrawDragIcon);
        });
        frame_clock.begin_updating();
        Ok(Self { surface: drag_surface })
    }

    pub fn request_redraw(&self) {
        self.surface.queue_render();
    }
}
