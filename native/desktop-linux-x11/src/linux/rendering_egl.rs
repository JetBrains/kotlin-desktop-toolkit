use crate::linux::{application_state::EglInstance, events::SoftwareDrawData, geometry::PhysicalSize};
use anyhow::{Context, bail};
use khronos_egl as egl;
use khronos_egl::NativeWindowType;
use log::{debug, info};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle};
use std::rc::Rc;
use winit_core::window::Window as WinitWindow;

#[derive(Debug)]
pub struct EglRendering<'a> {
    egl: &'a EglInstance,
    w: Rc<Box<dyn WinitWindow>>,
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
    pub fn new(egl: &'a EglInstance, w: Rc<Box<dyn WinitWindow>>) -> anyhow::Result<Self> {
        info!("Trying to use EGL rendering");

        let display_handle = w.display_handle().context("Failed to get raw window raw display handle")?;
        let display_handle_ptr = match display_handle.as_raw() {
            RawDisplayHandle::Xlib(handle) => handle.display.context("")?,
            // RawDisplayHandle::Xcb(handle) => {
            //     handle.connection.dis.context("")?
            // }
            _ => {
                bail!("Unsupported display handle type: {display_handle:?}");
            }
        };

        let egl_display = unsafe { egl.get_display(display_handle_ptr.as_ptr()) }.context("egl.get_display")?;
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

        let window_handle = w.window_handle()?;
        let window_handle_ptr = match window_handle.as_raw() {
            RawWindowHandle::Xlib(handle) => handle.window as NativeWindowType,
            RawWindowHandle::Xcb(handle) => handle.window.get() as NativeWindowType,
            _ => {
                bail!("Unsupported window handle type: {window_handle:?}");
            }
        };
        let egl_window_surface =
            unsafe { egl.create_window_surface(egl_display, egl_config, window_handle_ptr, None) }.context("egl.create_window_surface")?;

        egl.make_current(egl_display, Some(egl_window_surface), Some(egl_window_surface), Some(egl_context))
            .context("egl.make_current")?;

        Ok(Self {
            egl,
            w,
            egl_display,
            egl_window_surface,
            egl_context,
        })
    }

    pub const fn resize(&self, size: PhysicalSize) {
        // self.wl_egl_surface.resize(size.width.0, size.height.0, 0, 0);
    }

    pub fn draw<F>(&self, do_draw: F)
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
            self.w.pre_present_notify();
            self.egl
                .swap_buffers(self.egl_display, self.egl_window_surface)
                .with_context(|| format!("{:?}", self.w.id()))
                .unwrap();
        }
    }
}
