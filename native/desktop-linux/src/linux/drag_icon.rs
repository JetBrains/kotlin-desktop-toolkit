use crate::linux::{
    application_state::{ApplicationState, EGLData},
    events::{DragIconDrawEvent, SoftwareDrawData},
    geometry::{LogicalSize, PhysicalSize, Scale},
    rendering_egl::EglRendering,
    rendering_software::SoftwareRendering,
    window::RenderingData,
};
use log::{debug, info, trace, warn};
use smithay_client_toolkit::{
    compositor::{FrameCallbackData, Surface},
    reexports::{
        client::{Proxy as _, QueueHandle},
        protocols::wp::viewporter::client::wp_viewport::WpViewport,
    },
    shm::Shm,
};
use std::rc::Rc;

pub struct DragIcon {
    pub size: LogicalSize,
    viewport: Option<WpViewport>,
    pub surface: Surface,
    pub current_scale: Scale,
    rendering_data: RenderingData,
}

impl Drop for DragIcon {
    fn drop(&mut self) {
        warn!("DragIcon::drop: {}", self.surface.wl_surface().id());
    }
}

impl DragIcon {
    pub fn new(
        state: &ApplicationState,
        qh: &QueueHandle<ApplicationState>,
        shm: &Shm,
        size: LogicalSize,
        egl: Option<Rc<EGLData>>,
    ) -> anyhow::Result<Self> {
        debug!("DragIcon::new start: size={size:?}");
        let surface = Surface::new(&state.compositor_state, qh)?;
        let wl_surface = surface.wl_surface();
        let surface_id = wl_surface.id();

        debug!("DragIcon::new: wl_surface={surface_id:?}");

        if let Some(fractional_scale_manager) = state.fractional_scale_manager.as_ref() {
            fractional_scale_manager.get_fractional_scale(wl_surface, qh, surface_id);
        }

        let viewport = state.viewporter.as_ref().map(|vp| vp.get_viewport(wl_surface, qh, ()));
        let current_scale = Scale::default();

        let physical_size = size.to_rounded_physical(current_scale);

        let rendering_data = if let Some(egl) = egl {
            match EglRendering::new(egl, wl_surface, physical_size) {
                Ok(egl_rendering_data) => RenderingData::Egl(egl_rendering_data),
                Err(e) => {
                    warn!("Failed to create EGL rendering, falling back to software rendering. Error: {e:?}");
                    RenderingData::Software(SoftwareRendering::new(shm, physical_size)?)
                }
            }
        } else {
            info!("Forcing software rendering");
            RenderingData::Software(SoftwareRendering::new(shm, physical_size)?)
        };

        let mut icon = Self {
            size,
            viewport,
            surface,
            current_scale,
            rendering_data,
        };
        icon.on_resize(physical_size, shm);
        debug!("DragIcon::new finished");

        Ok(icon)
    }

    pub fn draw(&mut self, qh: &QueueHandle<ApplicationState>, callback: &dyn Fn(DragIconDrawEvent) -> bool) {
        trace!("DragIcon::draw");
        let wl_surface = self.surface.wl_surface();

        let physical_size = self.size.to_rounded_physical(self.current_scale);

        self.rendering_data
            .draw(wl_surface, physical_size, |software_draw_data: SoftwareDrawData| {
                let did_draw = callback(DragIconDrawEvent {
                    software_draw_data,
                    physical_size,
                    scale: self.current_scale,
                });

                if did_draw {
                    // Damage the entire window
                    wl_surface.damage_buffer(0, 0, physical_size.width.raw_physical(), physical_size.height.raw_physical());
                }

                // Request our next frame
                wl_surface.frame(qh, FrameCallbackData(wl_surface.clone()));
                did_draw
            });

        wl_surface.commit();
    }

    fn on_resize(&mut self, physical_size: PhysicalSize, shm: &Shm) {
        let size = self.size;
        if let Some(viewport) = &self.viewport {
            debug!("viewport.set_destination({:?}, {:?})", size.width, size.height);
            viewport.set_destination(size.width.raw_logical(), size.height.raw_logical());
        } else {
            let surface = self.surface.wl_surface();
            let buffer_scale = self.current_scale.to_scale_factor();
            debug!("surface.set_buffer_scale({buffer_scale})");
            surface.set_buffer_scale(buffer_scale);
        }

        match &mut self.rendering_data {
            RenderingData::Egl(egl_data) => {
                egl_data.resize(physical_size);
            }
            RenderingData::Software(data) => {
                if let Err(e) = data.resize(shm, physical_size) {
                    warn!("Error resizing software renderer for drag icon: {e}");
                }
            }
        }
    }

    pub fn scale_changed(&mut self, new_scale: Scale, shm: &Shm) {
        debug!("scale_changed: {new_scale:?}");
        self.current_scale = new_scale;
        self.on_resize(self.size.to_rounded_physical(self.current_scale), shm);
    }
}
