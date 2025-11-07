use crate::linux::{events::SoftwareDrawData, geometry::PhysicalSize};
use anyhow::anyhow;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::rc::Rc;
use winit_core::window::Window as WinitWindow;

pub struct SoftwareRendering {
    #[allow(clippy::type_complexity)]
    surface: Surface<Rc<Box<dyn WinitWindow>>, Rc<Box<dyn WinitWindow>>>,
}

impl SoftwareRendering {
    /// cbindgen:ignore
    const BYTES_PER_PIXEL: i32 = 4;

    pub fn new(w: Rc<Box<dyn WinitWindow>>) -> anyhow::Result<Self> {
        let context = Context::new(w.clone()).map_err(|e| anyhow!("Failed to create software rendering context: {e}"))?;
        let surface = Surface::new(&context, w).map_err(|e| anyhow!("Failed to create software rendering surface: {e}"))?;
        Ok(Self { surface })
    }

    pub fn resize(&mut self, size: PhysicalSize) {
        let w = NonZeroU32::new(u32::try_from(size.width.0).expect("Width must not be negative")).expect("Width must not be zero");
        let h = NonZeroU32::new(u32::try_from(size.height.0).expect("Height must not be negative")).expect("Height must not be zero");
        self.surface.resize(w, h).expect("Failed to resize software rendering surface");
    }

    pub fn draw<F>(&mut self, size: PhysicalSize, do_draw: F)
    where
        F: FnOnce(SoftwareDrawData) -> bool,
    {
        self.resize(size);
        let mut buffer = self.surface.buffer_mut().unwrap();
        let stride = size.width.0 * Self::BYTES_PER_PIXEL;
        let draw_data = SoftwareDrawData {
            canvas: buffer.as_mut_ptr().cast::<u8>(),
            stride,
        };
        if do_draw(draw_data) {
            buffer.present().unwrap();
        }
    }
}
