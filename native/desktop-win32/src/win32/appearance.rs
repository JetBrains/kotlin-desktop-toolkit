use desktop_common::logger::PanicDefault;
use windows::{
    UI::{
        Color as WUColor,
        ViewManagement::{UIColorType, UISettings},
    },
    core::Result as WinResult,
};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
    pub(crate) fn get_current() -> WinResult<Self> {
        let ui_settings = Self::cached_ui_settings()?;
        Self::from_ui_settings(&ui_settings)
    }

    // see https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-windows-themes#know-when-dark-mode-is-enabled
    fn from_ui_settings(settings: &UISettings) -> WinResult<Self> {
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

// see https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-windows-themes#know-when-dark-mode-is-enabled
#[inline]
const fn is_color_light(clr: WUColor) -> bool {
    ((5 * clr.G as u32) + (2 * clr.R as u32) + clr.B as u32) > (8 * 128)
}
