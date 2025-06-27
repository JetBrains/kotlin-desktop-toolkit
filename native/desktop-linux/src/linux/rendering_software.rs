use smithay_client_toolkit::{
    reexports::client::protocol::{wl_shm, wl_surface::WlSurface},
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};

use crate::linux::{events::SoftwareDrawData, geometry::PhysicalSize};

#[derive(Debug)]
pub struct SoftwareRendering {
    pool: SlotPool,
    buffer: Buffer,
    stride: i32,
}

struct SoftwareBuffer<'a> {
    buffer: Buffer,
    canvas: &'a mut [u8],
}

impl SoftwareRendering {
    const BYTES_PER_PIXEL: u8 = 4;

    fn create_buffer(pool: &mut SlotPool, size: PhysicalSize) -> SoftwareBuffer {
        let stride = size.width.0 * i32::from(Self::BYTES_PER_PIXEL);
        let (buffer, canvas) = pool
            .create_buffer(size.width.0, size.height.0, stride, wl_shm::Format::Argb8888)
            .expect("create buffer");
        SoftwareBuffer { buffer, canvas }
    }

    pub fn new(shm: &Shm, size: PhysicalSize) -> Self {
        let stride = size.width.0 * i32::from(Self::BYTES_PER_PIXEL);
        let mut pool = SlotPool::new(
            (stride * size.height.0 * 2).try_into().unwrap(), // double buffered
            shm,
        )
        .expect("Failed to create pool");
        let buffer = Self::create_buffer(&mut pool, size).buffer;
        Self { pool, buffer, stride }
    }

    pub fn resize(&mut self, shm: &Shm, size: PhysicalSize) {
        let stride = size.width.0 * i32::from(Self::BYTES_PER_PIXEL);
        if self.buffer.height() != size.height.0 || self.buffer.stride() != stride {
            *self = Self::new(shm, size);
        }
    }

    pub fn draw<F: FnOnce(Option<SoftwareDrawData>) -> bool>(&mut self, surface: &WlSurface, size: PhysicalSize, do_draw: F) {
        let canvas = if let Some(canvas) = self.pool.canvas(&self.buffer) {
            canvas
        } else {
            // This should be rare, but if the compositor has not released the previous
            // buffer, we need double-buffering.
            let second_draw_data = Self::create_buffer(&mut self.pool, size);
            self.buffer = second_draw_data.buffer;
            second_draw_data.canvas
        };

        let draw_data = Some(SoftwareDrawData {
            canvas: canvas.as_mut_ptr(),
            stride: self.stride,
        });
        if do_draw(draw_data) {
            self.buffer.attach_to(surface).expect("buffer attach");
        }
    }
}
