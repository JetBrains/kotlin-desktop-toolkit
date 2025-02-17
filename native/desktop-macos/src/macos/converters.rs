use objc2::rc::Retained;
use objc2_app_kit::NSColor;
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::common::{Color, LogicalPixels, LogicalPoint, LogicalRect, LogicalSize, PhysicalPoint, PhysicalSize};

impl From<NSSize> for LogicalSize {
    fn from(value: NSSize) -> Self {
        LogicalSize {
            width: value.width,
            height: value.height,
        }
    }
}

impl From<NSSize> for PhysicalSize {
    fn from(value: NSSize) -> Self {
        PhysicalSize {
            width: value.width,
            height: value.height,
        }
    }
}

impl From<NSPoint> for PhysicalPoint {
    fn from(value: NSPoint) -> Self {
        PhysicalPoint { x: value.x, y: value.y }
    }
}

impl From<PhysicalPoint> for NSPoint {
    fn from(value: PhysicalPoint) -> Self {
        NSPoint { x: value.x, y: value.y }
    }
}

#[allow(dead_code)]
impl LogicalPoint {
    pub(crate) fn to_macos_coords(&self, height: LogicalPixels) -> NSPoint {
        NSPoint {
            x: self.x,
            y: height - self.y,
        }
    }

    pub(crate) fn from_macos_coords(value: NSPoint, height: LogicalPixels) -> LogicalPoint {
        LogicalPoint {
            x: value.x,
            y: height - value.y,
        }
    }
}

impl From<LogicalSize> for NSSize {
    fn from(value: LogicalSize) -> Self {
        NSSize {
            width: value.width,
            height: value.height,
        }
    }
}

impl From<PhysicalSize> for NSSize {
    fn from(value: PhysicalSize) -> Self {
        NSSize {
            width: value.width,
            height: value.height,
        }
    }
}

impl LogicalRect {
    pub(crate) fn to_macos_coords(&self, height: LogicalPixels) -> NSRect {
        let origin = NSPoint {
            x: self.origin.x,
            y: height - (self.origin.y + self.size.height),
        };
        NSRect {
            origin,
            size: self.size.into(),
        }
    }

    pub(crate) fn from_macos_coords(value: NSRect, height: LogicalPixels) -> LogicalRect {
        let origin = LogicalPoint {
            x: value.origin.x,
            y: height - (value.origin.y + value.size.height),
        };
        LogicalRect {
            origin,
            size: value.size.into(),
        }
    }
}

impl From<Color> for Retained<NSColor> {
    fn from(value: Color) -> Self {
        unsafe { NSColor::colorWithSRGBRed_green_blue_alpha(value.red, value.green, value.blue, value.alpha) }
    }
}
