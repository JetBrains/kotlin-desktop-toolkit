use anyhow::Context;
use khronos_egl as egl;
use log::info;
use smithay_client_toolkit::reexports::client::{Connection, Proxy, protocol::wl_surface::WlSurface};
use wayland_egl::WlEglSurface;

use crate::linux::{application_state::EglInstance, events::SoftwareDrawData, geometry::PhysicalSize};

#[derive(Debug)]
pub struct EglRendering {
    wl_egl_surface: WlEglSurface,
    egl_display: egl::Display,
    egl_window_surface: khronos_egl::Surface,
    egl_context: egl::Context,
}

impl EglRendering {
    pub fn new(conn: &Connection, egl: &EglInstance, surface: &WlSurface, size: PhysicalSize) -> anyhow::Result<Self> {
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
            egl_context,
        })
    }

    pub fn resize(&self, size: PhysicalSize) {
        self.wl_egl_surface.resize(size.width.0, size.height.0, 0, 0);
    }

    pub fn draw<F: FnOnce(Option<SoftwareDrawData>) -> bool>(&self, surface: &WlSurface, egl: &EglInstance, do_draw: F) {
        egl.make_current(
            self.egl_display,
            Some(self.egl_window_surface),
            Some(self.egl_window_surface),
            Some(self.egl_context),
        )
        .context("egl.make_current")
        .unwrap();

        if do_draw(None) {
            egl.swap_buffers(self.egl_display, self.egl_window_surface)
                .context(surface.id())
                .unwrap();
        }
    }
}
