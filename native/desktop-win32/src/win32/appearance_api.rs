use desktop_common::logger::ffi_boundary;

use super::appearance::Appearance;

#[unsafe(no_mangle)]
pub extern "C" fn application_get_appearance() -> Appearance {
    ffi_boundary("application_get_appearance", || {
        let appearance = Appearance::get_current()?;
        Ok(appearance)
    })
}
