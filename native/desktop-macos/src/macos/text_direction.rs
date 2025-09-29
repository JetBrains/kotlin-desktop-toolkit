use desktop_common::logger::PanicDefault;
use objc2_app_kit::NSUserInterfaceLayoutDirection;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl PanicDefault for TextDirection {
    fn default() -> Self {
        Self::LeftToRight
    }
}

impl TextDirection {
    #[must_use]
    pub const fn from_ns_layout_direction(direction: NSUserInterfaceLayoutDirection) -> Self {
        match direction {
            NSUserInterfaceLayoutDirection::LeftToRight => Self::LeftToRight,
            NSUserInterfaceLayoutDirection::RightToLeft => Self::RightToLeft,
            _ => Self::LeftToRight, // Default to left-to-right for unknown values
        }
    }
}
