use desktop_common::logger::PanicDefault;
use objc2::rc::Retained;
use objc2_app_kit::{NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua};
use objc2_foundation::NSArray;

#[repr(C)]
#[derive(Debug)]
pub enum Appearance {
    Dark,
    Light,
}

impl PanicDefault for Appearance {
    fn default() -> Self {
        Self::Light
    }
}

impl Appearance {
    pub fn from_ns_appearance(value: &NSAppearance) -> Self {
        let light_name = unsafe { NSAppearanceNameAqua };
        let dark_name = unsafe { NSAppearanceNameDarkAqua };
        let options_array = NSArray::from_slice(&[light_name, dark_name]);
        let appearance_name = value
            .bestMatchFromAppearancesWithNames(&options_array)
            .expect("Unexpected appearance");
        match &*appearance_name {
            x if (x == light_name) => Self::Light,
            x if (x == dark_name) => Self::Dark,
            _ => unreachable!(),
        }
    }

    pub fn to_ns_appearance(&self) -> Retained<NSAppearance> {
        match self {
            Self::Dark => unsafe { NSAppearance::appearanceNamed(NSAppearanceNameDarkAqua) },
            Self::Light => unsafe { NSAppearance::appearanceNamed(NSAppearanceNameAqua) },
        }
        .expect("Failed to create appearance")
    }
}
