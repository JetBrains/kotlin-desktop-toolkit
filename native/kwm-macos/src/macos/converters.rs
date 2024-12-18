use objc2_foundation::{CGPoint, CGSize};

use crate::common::{LogicalPoint, LogicalSize, PhysicalPoint, PhysicalSize};


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