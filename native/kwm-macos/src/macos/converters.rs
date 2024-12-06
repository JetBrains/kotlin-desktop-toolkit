use objc2_foundation::CGSize;

use crate::common::Size;


impl From<CGSize> for Size {
    fn from(value: CGSize) -> Self {
        Size {
            width: value.width,
            height: value.height
        }
    }
}