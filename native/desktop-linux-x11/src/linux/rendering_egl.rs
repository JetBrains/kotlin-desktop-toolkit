use crate::linux::events::OpenGlDrawData;
use crate::linux::geometry::LogicalSize;
use gdk4::prelude::{DrawContextExt, GLContextExt, SurfaceExt};
use gtk4::gdk as gdk4;

pub struct EglRendering {
    gl_context: gdk4::GLContext,
}

impl EglRendering {
    pub fn new(surface: &gdk4::Surface) -> anyhow::Result<Self> {
        let gl_context = surface.create_gl_context()?;
        gl_context.set_required_version(3, 0);
        Ok(Self { gl_context })
    }

    pub fn draw<F>(&self, size: LogicalSize, do_draw: F)
    where
        F: FnOnce(OpenGlDrawData),
    {
        self.gl_context.make_current();
        let rect = gdk4::cairo::RectangleInt::new(0, 0, size.width, size.height);
        let region = gdk4::cairo::Region::create_rectangle(&rect);
        self.gl_context.begin_frame(&region);
        do_draw(OpenGlDrawData::default());
        self.gl_context.end_frame();
    }
}
