use gtk4::{gdk as gdk4, graphene};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PhysicalPixels(pub i32);

#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct LogicalPixels(pub f64);

#[repr(C)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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

impl From<(i32, i32)> for LogicalPoint {
    fn from(value: (i32, i32)) -> Self {
        Self {
            x: LogicalPixels(value.0.into()),
            y: LogicalPixels(value.1.into()),
        }
    }
}

impl From<(f64, f64)> for LogicalPoint {
    fn from(value: (f64, f64)) -> Self {
        Self {
            x: LogicalPixels(value.0),
            y: LogicalPixels(value.1),
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LogicalSize {
    pub width: i32,
    pub height: i32,
}
//
// #[must_use]
// pub fn to_logical_value<T: Into<f64>>(value: T, scale: f64) -> LogicalPixels {
//     LogicalPixels(value.into() / scale)
// }

impl From<LogicalSize> for graphene::Rect {
    fn from(value: LogicalSize) -> Self {
        #[allow(clippy::cast_precision_loss)]
        Self::new(0., 0., value.width as f32, value.height as f32)
    }
}

#[must_use]
fn to_physical_value(value: i32, scale: f64) -> PhysicalPixels {
    #[allow(clippy::cast_possible_truncation)]
    PhysicalPixels((f64::from(value) * scale).round() as i32)
}

#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub const fn round_to_i32(value: f64) -> i32 {
    value.round() as i32
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
pub struct PhysicalRect {
    pub x: PhysicalPixels,
    pub y: PhysicalPixels,
    pub width: PhysicalPixels,
    pub height: PhysicalPixels,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LogicalRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<LogicalRect> for gdk4::Rectangle {
    fn from(value: LogicalRect) -> Self {
        Self::new(value.x, value.y, value.width, value.height)
    }
}

impl PhysicalRect {
    // #[must_use]
    // pub fn to_logical(&self, scale: f64) -> LogicalRect {
    //     LogicalRect {
    //         x: to_logical_value(self.x.0, scale),
    //         y: to_logical_value(self.y.0, scale),
    //         width: to_logical_value(self.width.0, scale),
    //         height: to_logical_value(self.height.0, scale),
    //     }
    // }

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
            x: to_physical_value(self.x, scale),
            y: to_physical_value(self.y, scale),
            width: to_physical_value(self.width, scale),
            height: to_physical_value(self.height, scale),
        }
    }

    #[must_use]
    pub const fn as_size(&self) -> LogicalSize {
        LogicalSize {
            width: self.width,
            height: self.height,
        }
    }
}

// impl PhysicalSize {
//     #[must_use]
//     pub fn to_logical(&self, scale: f64) -> LogicalSize {
//         LogicalSize {
//             width: to_logical_value(self.width.0, scale),
//             height: to_logical_value(self.height.0, scale),
//         }
//     }
// }
