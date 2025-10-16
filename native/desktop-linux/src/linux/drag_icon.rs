use log::{debug, info, trace, warn};
use smithay_client_toolkit::{
    compositor::Surface,
    reexports::{
        client::{Proxy as _, QueueHandle, protocol::wl_display::WlDisplay},
        protocols::wp::viewporter::client::wp_viewport::WpViewport,
    },
    shm::Shm,
};

use crate::linux::{
    application_state::{ApplicationState, EglInstance},
    events::{DragIconDrawEvent, SoftwareDrawData},
    geometry::{LogicalSize, PhysicalSize},
    rendering_egl::EglRendering,
    rendering_software::SoftwareRendering,
    window::RenderingData,
};

pub struct DragIcon {
    pub size: LogicalSize,
    viewport: Option<WpViewport>,
    pub surface: Surface,
    pub current_scale: f64,
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
        wl_display: &WlDisplay,
        size: LogicalSize,
        egl: Option<&'static EglInstance>,
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
        let current_scale = 1.0;

        let physical_size = size.to_physical(current_scale);

        let rendering_data = if let Some(egl) = egl {
            match EglRendering::new(egl, wl_display, wl_surface, physical_size) {
                Ok(egl_rendering_data) => RenderingData::Egl(egl_rendering_data),
                Err(e) => {
                    warn!("Failed to create EGL rendering, falling back to software rendering. Error: {e:?}");
                    RenderingData::Software(SoftwareRendering::new(shm, physical_size))
                }
            }
        } else {
            info!("Forcing software rendering");
            RenderingData::Software(SoftwareRendering::new(shm, physical_size))
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

        let physical_size = self.size.to_physical(self.current_scale);

        self.rendering_data
            .draw(wl_surface, physical_size, |software_draw_data: SoftwareDrawData| {
                let did_draw = callback(DragIconDrawEvent {
                    software_draw_data,
                    physical_size,
                    scale: self.current_scale,
                });

                if did_draw {
                    // Damage the entire window
                    wl_surface.damage_buffer(0, 0, physical_size.width.0, physical_size.height.0);
                }

                // Request our next frame
                wl_surface.frame(qh, wl_surface.clone());
                did_draw
            });

        wl_surface.commit();
    }

    fn on_resize(&mut self, physical_size: PhysicalSize, shm: &Shm) {
        let size = self.size;
        if let Some(viewport) = &self.viewport {
            debug!("viewport.set_destination({}, {})", size.width, size.height);
            viewport.set_destination(size.width, size.height);
        } else {
            let surface = self.surface.wl_surface();
            assert!(self.current_scale % 1.0 < 0.0001);
            debug!("surface.set_buffer_scale({})", self.current_scale);
            #[allow(clippy::cast_possible_truncation)]
            surface.set_buffer_scale(self.current_scale as i32);
        }

        match &mut self.rendering_data {
            RenderingData::Egl(egl_data) => {
                egl_data.resize(physical_size);
            }
            RenderingData::Software(data) => {
                data.resize(shm, physical_size);
            }
        }
    }

    pub fn scale_changed(&mut self, new_scale: f64, shm: &Shm) {
        debug!("scale_changed: {new_scale}");
        self.current_scale = new_scale;
        self.on_resize(self.size.to_physical(self.current_scale), shm);
    }
}
