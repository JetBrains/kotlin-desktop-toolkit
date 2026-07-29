use anyhow::Context;
use log::warn;
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
    fn size_to_stride(size: PhysicalSize) -> anyhow::Result<i32> {
        const BYTES_PER_PIXEL: i32 = 4;
        size.width.raw_physical().checked_mul(BYTES_PER_PIXEL).context("stride")
    }

    fn create_buffer(pool: &mut SlotPool, size: PhysicalSize) -> anyhow::Result<SoftwareBuffer<'_>> {
        let stride = Self::size_to_stride(size)?;
        let (buffer, canvas) = pool.create_buffer(
            size.width.raw_physical(),
            size.height.raw_physical(),
            stride,
            wl_shm::Format::Argb8888,
        )?;
        Ok(SoftwareBuffer { buffer, canvas })
    }

    pub fn new(shm: &Shm, size: PhysicalSize) -> anyhow::Result<Self> {
        let stride = Self::size_to_stride(size)?;
        let mut pool = SlotPool::new(
            usize::try_from(stride * size.height.raw_physical() * 2)?, // double buffered
            shm,
        )?;
        let buffer = Self::create_buffer(&mut pool, size)?.buffer;
        Ok(Self { pool, buffer, stride })
    }

    pub fn resize(&mut self, shm: &Shm, size: PhysicalSize) -> anyhow::Result<()> {
        let stride = Self::size_to_stride(size)?;
        if self.buffer.height() != size.height.raw_physical() || self.buffer.stride() != stride {
            *self = Self::new(shm, size)?;
        }
        Ok(())
    }

    pub fn draw<F>(&mut self, surface: &WlSurface, size: PhysicalSize, do_draw: F)
    where
        F: FnOnce(SoftwareDrawData) -> bool,
    {
        let canvas = if let Some(canvas) = self.pool.canvas(&self.buffer) {
            canvas
        } else {
            // This should be rare, but if the compositor has not released the previous
            // buffer, we need double-buffering.
            match Self::create_buffer(&mut self.pool, size) {
                Ok(second_draw_data) => {
                    self.buffer = second_draw_data.buffer;
                    second_draw_data.canvas
                }
                Err(e) => {
                    warn!("Failed to create second buffer: {e}");
                    return;
                }
            }
        };

        let draw_data = SoftwareDrawData {
            canvas: canvas.as_mut_ptr(),
            stride: self.stride,
        };
        if do_draw(draw_data) {
            self.buffer.attach_to(surface).expect("buffer attach");
        }
    }
}
