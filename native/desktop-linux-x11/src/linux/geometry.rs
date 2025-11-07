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

#[allow(clippy::fallible_impl_from)]
impl From<dpi::PhysicalSize<u32>> for PhysicalSize {
    fn from(size: dpi::PhysicalSize<u32>) -> Self {
        Self {
            width: PhysicalPixels(i32::try_from(size.width).unwrap()),
            height: PhysicalPixels(i32::try_from(size.height).unwrap()),
        }
    }
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

impl From<dpi::LogicalPosition<f64>> for LogicalPoint {
    fn from(value: dpi::LogicalPosition<f64>) -> Self {
        Self {
            x: LogicalPixels(value.x),
            y: LogicalPixels(value.y),
        }
    }
}

impl LogicalPoint {
    #[must_use]
    pub const fn as_winit_position(&self) -> dpi::Position {
        dpi::Position::Logical(dpi::LogicalPosition { x: self.x.0, y: self.y.0 })
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct LogicalSize {
    pub width: LogicalPixels,
    pub height: LogicalPixels,
}

impl From<dpi::LogicalSize<f64>> for LogicalSize {
    fn from(value: dpi::LogicalSize<f64>) -> Self {
        Self {
            width: LogicalPixels(value.width),
            height: LogicalPixels(value.height),
        }
    }
}

#[must_use]
pub fn to_logical_value<T: Into<f64>>(value: T, scale: f64) -> LogicalPixels {
    LogicalPixels(value.into() / scale)
}

#[must_use]
fn to_physical_value(value: f64, scale: f64) -> PhysicalPixels {
    #[allow(clippy::cast_possible_truncation)]
    PhysicalPixels((value * scale).round() as i32)
}

#[must_use]
pub fn round_to_u32(value: f64) -> u32 {
    u32::try_from(value.round() as i64).unwrap()
}

impl LogicalSize {
    #[must_use]
    pub fn to_physical(&self, scale: f64) -> PhysicalSize {
        PhysicalSize {
            width: to_physical_value(self.width.0, scale),
            height: to_physical_value(self.height.0, scale),
        }
    }

    #[must_use]
    pub const fn as_winit_size(&self) -> dpi::Size {
        dpi::Size::Logical(dpi::LogicalSize {
            width: self.width.0,
            height: self.height.0,
        })
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PhysicalRect {
    pub x: PhysicalPixels,
    pub y: PhysicalPixels,
    pub width: PhysicalPixels,
    pub height: PhysicalPixels,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalRect {
    pub x: LogicalPixels,
    pub y: LogicalPixels,
    pub width: LogicalPixels,
    pub height: LogicalPixels,
}

impl PhysicalRect {
    #[must_use]
    pub fn to_logical(&self, scale: f64) -> LogicalRect {
        LogicalRect {
            x: to_logical_value(self.x.0, scale),
            y: to_logical_value(self.y.0, scale),
            width: to_logical_value(self.width.0, scale),
            height: to_logical_value(self.height.0, scale),
        }
    }

    #[must_use]
    pub const fn as_size(&self) -> PhysicalSize {
        PhysicalSize {
            width: self.width,
            height: self.height,
        }
    }
}

impl LogicalRect {
    #[must_use]
    pub fn to_physical(&self, scale: f64) -> PhysicalRect {
        PhysicalRect {
            x: to_physical_value(self.x.0, scale),
            y: to_physical_value(self.y.0, scale),
            width: to_physical_value(self.width.0, scale),
            height: to_physical_value(self.height.0, scale),
        }
    }

    #[must_use]
    pub const fn as_size(&self) -> LogicalSize {
        LogicalSize {
            width: self.width,
            height: self.height,
        }
    }

    #[must_use]
    pub const fn as_winit_position(&self) -> dpi::LogicalPosition<f64> {
        dpi::LogicalPosition { x: self.x.0, y: self.y.0 }
    }

    #[must_use]
    pub const fn as_winit_size(&self) -> dpi::LogicalSize<f64> {
        dpi::LogicalSize {
            width: self.width.0,
            height: self.height.0,
        }
    }
}

impl PhysicalSize {
    #[must_use]
    pub fn to_logical(&self, scale: f64) -> LogicalSize {
        LogicalSize {
            width: to_logical_value(self.width.0, scale),
            height: to_logical_value(self.height.0, scale),
        }
    }
}
