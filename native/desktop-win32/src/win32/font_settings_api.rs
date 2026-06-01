use desktop_common::logger::ffi_boundary;

use super::font_settings::{
    FontSmoothing, FontSmoothingOrientation, FontSmoothingType, get_font_smoothing_contrast,
};

#[unsafe(no_mangle)]
pub extern "C" fn application_get_font_smoothing() -> FontSmoothing {
    ffi_boundary("application_get_font_smoothing", FontSmoothing::get_current)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_font_smoothing_type() -> FontSmoothingType {
    ffi_boundary("application_get_font_smoothing_type", FontSmoothingType::get_current)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_font_smoothing_contrast() -> u32 {
    ffi_boundary("application_get_font_smoothing_contrast", get_font_smoothing_contrast)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_font_smoothing_orientation() -> FontSmoothingOrientation {
    ffi_boundary(
        "application_get_font_smoothing_orientation",
        FontSmoothingOrientation::get_current,
    )
}
