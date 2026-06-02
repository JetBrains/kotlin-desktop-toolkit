use std::sync::OnceLock;

use desktop_common::logger::PanicDefault;
use windows::{
    UI::{
        Color as WUColor,
        ViewManagement::{UIColorType, UISettings},
    },
    Win32::{
        Foundation::HMODULE,
        System::LibraryLoader::{GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW},
        UI::{
            Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW, HIGHCONTRASTW_FLAGS},
            WindowsAndMessaging::{SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW},
        },
    },
};
use windows_core::{PCSTR, PWSTR, Result as WinResult, w};

use super::utils;

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

type RawProcFn = unsafe extern "system" fn() -> isize;
// Raw `i32` both ways: pass `PreferredAppMode as i32` going in, feed the returned previous
// mode straight back when restoring, without rebuilding a `#[repr(i32)]` enum from it.
type SetPreferredAppModeFn = unsafe extern "system" fn(i32) -> i32;
type FlushMenuThemesFn = unsafe extern "system" fn();

#[repr(i32)]
#[allow(dead_code)]
enum PreferredAppMode {
    Default = 0,
    AllowDark = 1,
    ForceDark = 2,
    ForceLight = 3,
    Max = 4,
}

/// `uxtheme.dll` dark-mode ordinals #135 and #136, resolved as a pair (both shipped
/// in Windows 10 1903, below our 22000 gate).
struct UxThemeFns {
    set_preferred_app_mode: SetPreferredAppModeFn,
    flush_menu_themes: FlushMenuThemesFn,
}

/// Resolve and cache the ordinals once. `None` (logged, non-fatal) on pre-22000
/// Windows, DLL-load failure, or either ordinal missing — the popup then stays light.
fn ux_theme_fns() -> Option<&'static UxThemeFns> {
    static FNS: OnceLock<Option<UxThemeFns>> = OnceLock::new();
    FNS.get_or_init(|| {
        if !utils::is_windows_11_build_22000_or_higher() {
            return None;
        }
        let module = match unsafe { LoadLibraryExW(w!("uxtheme.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) } {
            Ok(module) => module,
            Err(err) => {
                log::warn!("LoadLibraryExW(uxtheme.dll) failed: {err}");
                return None;
            }
        };
        let set_preferred_app_mode = resolve_ordinal(module, 135, "SetPreferredAppMode")?;
        let flush_menu_themes = resolve_ordinal(module, 136, "FlushMenuThemes")?;
        Some(UxThemeFns {
            // SAFETY: ordinal #135 signature per Chromium base/win/dark_mode_support.cc
            // and ysc3839/win32-darkmode. Both stub and target are `extern "system"`.
            set_preferred_app_mode: unsafe { std::mem::transmute::<RawProcFn, SetPreferredAppModeFn>(set_preferred_app_mode) },
            // SAFETY: ordinal #136 signature per ysc3839/win32-darkmode (`void()`).
            flush_menu_themes: unsafe { std::mem::transmute::<RawProcFn, FlushMenuThemesFn>(flush_menu_themes) },
        })
    })
    .as_ref()
}

fn resolve_ordinal(module: HMODULE, n: u16, name: &str) -> Option<RawProcFn> {
    // MAKEINTRESOURCEA: the ordinal sits in the low word of an otherwise-null PCSTR.
    let ordinal = PCSTR(n as usize as *const u8);
    let raw = unsafe { GetProcAddress(module, ordinal) };
    if raw.is_none() {
        log::warn!("uxtheme ordinal #{n} ({name}) missing");
    }
    raw
}

/// Forces the menu palette, flushes cached menu themes, and captures the previous
/// process mode; the previous mode is restored (and flushed again) on drop.
struct PreferredAppModeGuard {
    fns: &'static UxThemeFns,
    previous: i32,
}

impl PreferredAppModeGuard {
    fn set(appearance: Appearance) -> Option<Self> {
        let fns = ux_theme_fns()?;
        let mode = match appearance {
            Appearance::Dark => PreferredAppMode::ForceDark,
            Appearance::Light => PreferredAppMode::ForceLight,
        };
        let previous = unsafe { (fns.set_preferred_app_mode)(mode as i32) };
        unsafe { (fns.flush_menu_themes)() };
        Some(Self { fns, previous })
    }
}

impl Drop for PreferredAppModeGuard {
    fn drop(&mut self) {
        unsafe { (self.fns.set_preferred_app_mode)(self.previous) };
        unsafe { (self.fns.flush_menu_themes)() };
    }
}

/// Runs `f` with the menu palette forced to `appearance`, restoring the previous
/// process mode afterward. On pre-22000 Windows (or resolution failure) the mode is
/// left untouched and `f` still runs, so the menu opens unthemed.
pub(crate) fn with_preferred_app_mode<R>(appearance: Appearance, f: impl FnOnce() -> R) -> R {
    let _guard = PreferredAppModeGuard::set(appearance);
    f()
}
