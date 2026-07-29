use anyhow::Context;
use khronos_egl as egl;
use log::{debug, info};
use smithay_client_toolkit::reexports::client::{Proxy as _, protocol::wl_surface::WlSurface};
use std::rc::Rc;
use wayland_egl::WlEglSurface;

use crate::linux::application_state::EGLData;
use crate::linux::{events::SoftwareDrawData, geometry::PhysicalSize};

pub struct EglRendering {
    egl_data: Rc<EGLData>,
    wl_egl_surface: WlEglSurface,
    egl_window_surface: khronos_egl::Surface,
    egl_context: egl::Context,
}

impl Drop for EglRendering {
    fn drop(&mut self) {
        debug!("EglRendering::drop");
        let egl = &self.egl_data.instance;
        let egl_display = self.egl_data.display;

        egl.make_current(egl_display, None, None, None).expect("eglMakeCurrent");

        if self.egl_context.as_ptr() != egl::NO_CONTEXT {
            egl.destroy_context(egl_display, self.egl_context).expect("eglDestroyContext");
        }

        if self.egl_window_surface.as_ptr() != egl::NO_SURFACE {
            egl.destroy_surface(egl_display, self.egl_window_surface)
                .expect("eglDestroySurface");
        }
    }
}

impl EglRendering {
    pub fn new(egl_data: Rc<EGLData>, surface: &WlSurface, size: PhysicalSize) -> anyhow::Result<Self> {
        info!("Trying to use EGL rendering for {}", surface.id());
        let egl = &egl_data.instance;
        let egl_display = egl_data.display;

        let wl_egl_surface = WlEglSurface::new(surface.id(), size.width.0, size.height.0)
            .with_context(|| format!("WlEglSurface::new (surface.id() = {})", surface.id()))?;

        let egl_attributes = [
            egl::SURFACE_TYPE,
            egl::WINDOW_BIT,
            egl::COLOR_BUFFER_TYPE,
            egl::RGB_BUFFER,
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
            egl_data,
            wl_egl_surface,
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
        let egl = &self.egl_data.instance;
        let egl_display = self.egl_data.display;

        egl.make_current(
            egl_display,
            Some(self.egl_window_surface),
            Some(self.egl_window_surface),
            Some(self.egl_context),
        )
        .context("egl.make_current")
        .unwrap();

        if do_draw(SoftwareDrawData::default()) {
            egl.swap_buffers(egl_display, self.egl_window_surface)
                .context(surface.id())
                .unwrap();
        }
    }
}
