#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPixels(pub i32);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalPixels(pub f32);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPoint {
    pub x: PhysicalPixels,
    pub y: PhysicalPixels,
}

impl PhysicalPoint {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self {
            x: PhysicalPixels(x),
            y: PhysicalPixels(y),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalSize {
    pub width: PhysicalPixels,
    pub height: PhysicalPixels,
}

impl PhysicalSize {
    #[must_use]
    pub const fn new(width: i32, height: i32) -> Self {
        Self {
            width: PhysicalPixels(width),
            height: PhysicalPixels(height),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalPoint {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalSize {
    pub width: LogicalPixels,
    pub height: LogicalPixels,
}

impl LogicalPoint {
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn to_physical(self, scale: f32) -> PhysicalPoint {
        PhysicalPoint::new(
            f32::floor(self.x.0.mul_add(scale, 0.5_f32)) as i32,
            f32::floor(self.y.0.mul_add(scale, 0.5_f32)) as i32,
        )
    }
}

impl LogicalSize {
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn to_physical(self, scale: f32) -> PhysicalSize {
        PhysicalSize::new(
            f32::floor(self.width.0.mul_add(scale, 0.5_f32)) as i32,
            f32::floor(self.height.0.mul_add(scale, 0.5_f32)) as i32,
        )
    }
}

impl PhysicalPoint {
    #[allow(clippy::cast_precision_loss)]
    pub(crate) const fn to_logical(self, scale: f32) -> LogicalPoint {
        // 0.5 is to ensure that we pixel snap correctly at the edges, this is necessary with odd DPIs like 1.25, 1.5, 1, .75
        LogicalPoint {
            x: LogicalPixels((self.x.0 as f32 / scale) + 0.5_f32),
            y: LogicalPixels((self.y.0 as f32 / scale) + 0.5_f32),
        }
    }
}
