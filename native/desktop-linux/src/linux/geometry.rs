#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalPixels(pub i32);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalPixels(pub f64);

impl LogicalPixels {
    #[allow(clippy::cast_possible_truncation)]
    #[must_use]
    pub const fn round(&self) -> i32 {
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
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalPoint {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
}

impl<T: Into<f64>> From<(T, T)> for LogicalPoint {
    fn from(value: (T, T)) -> Self {
        Self {
            x: LogicalPixels(value.0.into()),
            y: LogicalPixels(value.1.into()),
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalSize {
    pub width: i32,
    pub height: i32,
}

#[must_use]
fn to_physical_value(value: i32, scale: f64) -> PhysicalPixels {
    #[allow(clippy::cast_possible_truncation)]
    PhysicalPixels((f64::from(value) * scale).round() as i32)
}

impl LogicalSize {
    #[must_use]
    pub fn to_physical(&self, scale: f64) -> PhysicalSize {
        PhysicalSize {
            width: to_physical_value(self.width, scale),
            height: to_physical_value(self.height, scale),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
