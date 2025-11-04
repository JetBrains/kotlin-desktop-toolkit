use anyhow::Context;
use khronos_egl as egl;
use log::{debug, info};
use smithay_client_toolkit::reexports::client::{
    Proxy as _,
    protocol::{wl_display::WlDisplay, wl_surface::WlSurface},
};
use wayland_egl::WlEglSurface;

use crate::linux::{application_state::EglInstance, events::SoftwareDrawData, geometry::PhysicalSize};

#[derive(Debug)]
pub struct EglRendering<'a> {
    egl: &'a EglInstance,
    wl_egl_surface: WlEglSurface,
    egl_display: egl::Display,
    egl_window_surface: khronos_egl::Surface,
    egl_context: egl::Context,
}

impl Drop for EglRendering<'_> {
    fn drop(&mut self) {
        debug!("EglRendering::drop");
        self.egl.destroy_context(self.egl_display, self.egl_context).unwrap();
        self.egl.destroy_surface(self.egl_display, self.egl_window_surface).unwrap();
    }
}

impl<'a> EglRendering<'a> {
    pub fn new(egl: &'a EglInstance, display: &WlDisplay, surface: &WlSurface, size: PhysicalSize) -> anyhow::Result<Self> {
        info!("Trying to use EGL rendering for {}", surface.id());

        let wl_egl_surface = WlEglSurface::new(surface.id(), size.width.0, size.height.0)
            .with_context(|| format!("WlEglSurface::new (surface.id() = {})", surface.id()))?;

        let wayland_display_ptr = display.id().as_ptr();
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
            .with_context(|| format!("egl.create_window_surface, surface.id()={}", surface.id()))?;

        egl.make_current(egl_display, Some(egl_window_surface), Some(egl_window_surface), Some(egl_context))
            .context("egl.make_current")?;

        Ok(Self {
            egl,
            wl_egl_surface,
            egl_display,
            egl_window_surface,
            egl_context,
        })
    }

    pub fn resize(&self, size: PhysicalSize) {
        self.wl_egl_surface.resize(size.width.0, size.height.0, 0, 0);
    }

    pub fn draw<F>(&self, surface: &WlSurface, do_draw: F)
    where
        F: FnOnce(SoftwareDrawData) -> bool,
    {
        self.egl
            .make_current(
                self.egl_display,
                Some(self.egl_window_surface),
                Some(self.egl_window_surface),
                Some(self.egl_context),
            )
            .context("egl.make_current")
            .unwrap();

        if do_draw(SoftwareDrawData::default()) {
            self.egl
                .swap_buffers(self.egl_display, self.egl_window_surface)
                .context(surface.id())
                .unwrap();
        }
    }
}
