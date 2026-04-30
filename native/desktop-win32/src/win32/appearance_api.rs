use desktop_common::logger::ffi_boundary;

use super::appearance::{Appearance, HighContrast};

#[unsafe(no_mangle)]
pub extern "C" fn application_get_appearance() -> Appearance {
    ffi_boundary("application_get_appearance", Appearance::get_current)
}

#[unsafe(no_mangle)]
pub extern "C" fn application_get_high_contrast() -> HighContrast {
    ffi_boundary("application_get_high_contrast", HighContrast::get_current)
}
