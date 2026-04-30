use desktop_common::logger::PanicDefault;
use windows::{
    UI::{
        Color as WUColor,
        ViewManagement::{UIColorType, UISettings},
    },
    Win32::UI::{
        Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW, HIGHCONTRASTW_FLAGS},
        WindowsAndMessaging::{SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW},
    },
};
use windows_core::{PWSTR, Result as WinResult};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub(crate) fn get_current() -> anyhow::Result<Self> {
        let ui_settings = Self::cached_ui_settings()?;
        Self::from_ui_settings(&ui_settings)
    }

    // see https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-windows-themes#know-when-dark-mode-is-enabled
    fn from_ui_settings(settings: &UISettings) -> anyhow::Result<Self> {
        let foreground_color = settings.GetColorValue(UIColorType::Foreground)?;
        let is_dark_mode = is_color_light(foreground_color);
        if is_dark_mode { Ok(Self::Dark) } else { Ok(Self::Light) }
    }

    fn cached_ui_settings() -> WinResult<UISettings> {
        static CACHED_UI_SETTINGS: std::sync::OnceLock<UISettings> = std::sync::OnceLock::new();

        if let Some(ui_settings) = CACHED_UI_SETTINGS.get() {
            Ok(ui_settings.clone())
        } else {
            let ui_settings = UISettings::new()?;
            let cached = CACHED_UI_SETTINGS.get_or_init(|| ui_settings);
            Ok(cached.clone())
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighContrast {
    Off,
    On,
}

impl PanicDefault for HighContrast {
    fn default() -> Self {
        Self::Off
    }
}

impl HighContrast {
    pub(crate) fn get_current() -> anyhow::Result<Self> {
        let mut hc = HIGHCONTRASTW {
            cbSize: size_of::<HIGHCONTRASTW>().try_into()?,
            dwFlags: HIGHCONTRASTW_FLAGS(0),
            lpszDefaultScheme: PWSTR::null(),
        };
        unsafe {
            SystemParametersInfoW(
                SPI_GETHIGHCONTRAST,
                size_of::<HIGHCONTRASTW>().try_into()?,
                Some((&raw mut hc).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )?;
        }
        // Do NOT call LocalFree on hc.lpszDefaultScheme. The MS doc says
        // "The system allocates this buffer, free it with LocalFree", but
        // empirically that crashes the heap (NTSTATUS 0xC0000374). The
        // pointer is not a LocalAlloc'd heap allocation despite the doc.
        if hc.dwFlags.contains(HCF_HIGHCONTRASTON) {
            Ok(Self::On)
        } else {
            Ok(Self::Off)
        }
    }
}

// see https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-windows-themes#know-when-dark-mode-is-enabled
#[inline]
const fn is_color_light(clr: WUColor) -> bool {
    ((5 * clr.G as u32) + (2 * clr.R as u32) + clr.B as u32) > (8 * 128)
}
