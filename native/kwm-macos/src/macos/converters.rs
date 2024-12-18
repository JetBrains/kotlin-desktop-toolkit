use objc2_foundation::CGSize;

use crate::common::{LogicalSize, PhysicalSize};


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