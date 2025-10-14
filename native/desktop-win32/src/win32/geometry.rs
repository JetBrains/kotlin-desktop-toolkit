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

    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn to_logical(self, scale: f32) -> LogicalPoint {
        LogicalPoint::new((self.x.0 as f32) / scale, (self.y.0 as f32) / scale)
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

    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn to_logical(self, scale: f32) -> LogicalSize {
        LogicalSize::new((self.width.0 as f32) / scale, (self.height.0 as f32) / scale)
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalPoint {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

impl LogicalPoint {
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self {
            x: LogicalPixels(x),
            y: LogicalPixels(y),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn to_physical(self, scale: f32) -> PhysicalPoint {
        PhysicalPoint::new(
            f32::floor(self.x.0.mul_add(scale, 0.5_f32)) as i32,
            f32::floor(self.y.0.mul_add(scale, 0.5_f32)) as i32,
        )
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalSize {
    pub width: LogicalPixels,
    pub height: LogicalPixels,
}

impl LogicalSize {
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self {
            width: LogicalPixels(width),
            height: LogicalPixels(height),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn to_physical(self, scale: f32) -> PhysicalSize {
        PhysicalSize::new(
            f32::floor(self.width.0.mul_add(scale, 0.5_f32)) as i32,
            f32::floor(self.height.0.mul_add(scale, 0.5_f32)) as i32,
        )
    }
}
