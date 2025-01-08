use objc2::rc::Retained;
use objc2_app_kit::NSColor;
use objc2_foundation::{CGPoint, CGSize};

use crate::common::{Color, LogicalPoint, LogicalSize, PhysicalPoint, PhysicalSize};


impl From<CGSize> for LogicalSize {
    fn from(value: CGSize) -> Self {
        LogicalSize {
            width: value.width,
            height: value.height
        }
    }
}

impl From<CGSize> for PhysicalSize {
    fn from(value: CGSize) -> Self {
        PhysicalSize {
            width: value.width,
            height: value.height
        }
    }
}

impl From<CGPoint> for LogicalPoint {
    fn from(value: CGPoint) -> Self {
        LogicalPoint {
            x: value.x,
            y: value.y
        }
    }
}

impl From<CGPoint> for PhysicalPoint {
    fn from(value: CGPoint) -> Self {
        PhysicalPoint {
            x: value.x,
            y: value.y
        }
    }
}

impl From<LogicalPoint> for CGPoint {
    fn from(value: LogicalPoint) -> Self {
        CGPoint {
            x: value.x,
            y: value.y
        }
    }
}

impl From<PhysicalPoint> for CGPoint {
    fn from(value: PhysicalPoint) -> Self {
        CGPoint {
            x: value.x,
            y: value.y
        }
    }
}

impl From<LogicalSize> for CGSize {
    fn from(value: LogicalSize) -> Self {
        CGSize {
            width: value.width,
            height: value.height
        }
    }
}

impl From<PhysicalSize> for CGSize {
    fn from(value: PhysicalSize) -> Self {
        CGSize {
            width: value.width,
            height: value.height
        }
    }
}

impl From<Color> for Retained<NSColor> {
    fn from(value: Color) -> Self {
        unsafe {
            NSColor::colorWithSRGBRed_green_blue_alpha(value.red, value.green, value.blue, value.alpha)
        }
    }
}