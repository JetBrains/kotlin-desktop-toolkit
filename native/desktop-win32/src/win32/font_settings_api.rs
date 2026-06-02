use desktop_common::logger::{PanicDefault, ffi_boundary};

use super::font_settings::get_font_smoothing_contrast;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSmoothing {
    Disabled,
    Enabled,
}

impl PanicDefault for FontSmoothing {
    fn default() -> Self {
        Self::Disabled
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSmoothingType {
    Standard,
    ClearType,
}

impl PanicDefault for FontSmoothingType {
    fn default() -> Self {
        Self::Standard
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSmoothingOrientation {
    Rgb,
    Bgr,
}

impl PanicDefault for FontSmoothingOrientation {
    fn default() -> Self {
        Self::Rgb
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn font_settings_get_font_smoothing() -> FontSmoothing {
    ffi_boundary("font_settings_get_font_smoothing", FontSmoothing::get_current)
}

#[unsafe(no_mangle)]
pub extern "C" fn font_settings_get_font_smoothing_type() -> FontSmoothingType {
    ffi_boundary("font_settings_get_font_smoothing_type", FontSmoothingType::get_current)
}

#[unsafe(no_mangle)]
pub extern "C" fn font_settings_get_font_smoothing_contrast() -> u32 {
    ffi_boundary("font_settings_get_font_smoothing_contrast", get_font_smoothing_contrast)
}

#[unsafe(no_mangle)]
pub extern "C" fn font_settings_get_font_smoothing_orientation() -> FontSmoothingOrientation {
    ffi_boundary(
        "font_settings_get_font_smoothing_orientation",
        FontSmoothingOrientation::get_current,
    )
}
