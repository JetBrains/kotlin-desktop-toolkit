use anyhow::bail;
use gdk4::prelude::{Cast, DragExt, DragSurfaceExt, SurfaceExt};

use crate::linux::application::send_event;
use crate::linux::events::{Event, EventHandler};
use crate::linux::rendering_egl::EglRendering;
use crate::linux::{events::DragIconDrawEvent, geometry::LogicalSize};
use gtk4::gdk as gdk4;

pub struct DragIcon {
    pub surface: gdk4::DragSurface,
}

impl DragIcon {
    pub fn new(event_handler: EventHandler, drag: &gdk4::Drag, size: LogicalSize) -> anyhow::Result<Self> {
        let Some(drag_surface) = drag.drag_surface() else {
            bail!("Drag has no surface");
        };

        let rendering_data = EglRendering::new(&drag_surface).expect("Failed to create rendering data");
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
        drag_surface.frame_clock().connect_update(move |_| {
            send_event(event_handler, Event::ShouldRedrawDragIcon);
        });
        drag_surface.queue_render();
        Ok(Self { surface: drag_surface })
    }

    pub fn request_redraw(&self) {
        self.surface.queue_render();
    }
}
