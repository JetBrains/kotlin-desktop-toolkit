use windows::Win32::UI::WindowsAndMessaging::{
    FE_FONTSMOOTHINGCLEARTYPE, FE_FONTSMOOTHINGORIENTATIONBGR, FE_FONTSMOOTHINGORIENTATIONRGB, FE_FONTSMOOTHINGSTANDARD,
    SPI_GETFONTSMOOTHING, SPI_GETFONTSMOOTHINGCONTRAST, SPI_GETFONTSMOOTHINGORIENTATION, SPI_GETFONTSMOOTHINGTYPE,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};

use super::font_settings_api::{FontSmoothing, FontSmoothingOrientation, FontSmoothingType};

impl FontSmoothing {
    pub(crate) fn get_current() -> anyhow::Result<Self> {
        let mut enabled: u32 = 0;
        unsafe {
            SystemParametersInfoW(
                SPI_GETFONTSMOOTHING,
                0,
                Some((&raw mut enabled).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )?;
        }
        if enabled != 0 { Ok(Self::Enabled) } else { Ok(Self::Disabled) }
    }
}

impl FontSmoothingType {
    pub(crate) fn get_current() -> anyhow::Result<Self> {
        let mut smoothing_type: u32 = 0;
        unsafe {
            SystemParametersInfoW(
                SPI_GETFONTSMOOTHINGTYPE,
                0,
                Some((&raw mut smoothing_type).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )?;
        }
        match smoothing_type {
            v if v == FE_FONTSMOOTHINGCLEARTYPE => Ok(Self::ClearType),
            v if v == FE_FONTSMOOTHINGSTANDARD => Ok(Self::Standard),
            other => anyhow::bail!("unknown font smoothing type: {other}"),
        }
    }
}

pub(crate) fn get_font_smoothing_contrast() -> anyhow::Result<u32> {
    let mut contrast: u32 = 0;
    unsafe {
        SystemParametersInfoW(
            SPI_GETFONTSMOOTHINGCONTRAST,
            0,
            Some((&raw mut contrast).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )?;
    }
    Ok(contrast)
}

impl FontSmoothingOrientation {
    pub(crate) fn get_current() -> anyhow::Result<Self> {
        let mut orientation: u32 = 0;
        unsafe {
            SystemParametersInfoW(
                SPI_GETFONTSMOOTHINGORIENTATION,
                0,
                Some((&raw mut orientation).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )?;
        }
        match orientation {
            v if v == FE_FONTSMOOTHINGORIENTATIONBGR => Ok(Self::Bgr),
            v if v == FE_FONTSMOOTHINGORIENTATIONRGB => Ok(Self::Rgb),
            other => anyhow::bail!("unknown font smoothing orientation: {other}"),
        }
    }
}
