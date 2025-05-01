#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPixels(pub i32);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalPixels(pub f64);

impl LogicalPixels {
    #[must_use]
    pub fn to_physical(&self, scale: f64) -> PhysicalPixels {
        #[allow(clippy::cast_possible_truncation)]
        PhysicalPixels((self.0 * scale).round() as i32)
    }

    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    pub fn round(&self) -> i32 {
        self.0.round() as i32
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalSize {
    pub width: PhysicalPixels,
    pub height: PhysicalPixels,
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

impl LogicalSize {
    #[must_use]
    pub fn to_physical(&self, scale: f64) -> PhysicalSize {
        PhysicalSize {
            width: self.width.to_physical(scale),
            height: self.height.to_physical(scale),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalRect {
    pub origin: LogicalPoint,
    pub size: LogicalSize,
}
