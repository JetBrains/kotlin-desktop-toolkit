use anyhow::{Context, bail};
use ashpd::{
    desktop::settings::{ACCENT_COLOR_SCHEME_KEY, APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY, ColorScheme, Namespace, Settings},
    zvariant::{OwnedValue, Structure},
};
use desktop_common::ffi_utils::BorrowedArray;
use futures_lite::StreamExt;
use log::{debug, error};

use crate::linux::desktop_settings_api::{
    Color, DesktopTitlebarAction, FfiDesktopSetting, FontAntialiasing, FontHinting, FontRgbaOrder, XdgDesktopColorScheme,
};

/// cbindgen:ignore
const GNOME_DESKTOP_INTERFACE_NAMESPACE: &str = "org.gnome.desktop.interface";

/// cbindgen:ignore
const GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE: &str = "org.gnome.desktop.peripherals.mouse";

/// cbindgen:ignore
const GNOME_DESKTOP_PRIVACY_NAMESPACE: &str = "org.gnome.desktop.privacy";

/// cbindgen:ignore
const GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE: &str = "org.gnome.desktop.wm.preferences";

#[derive(Debug)]
pub enum InternalDesktopSetting {
    AccentColor(Color),
    AudibleBell(bool),
    ActionDoubleClickTitlebar(DesktopTitlebarAction),
    ActionRightClickTitlebar(DesktopTitlebarAction),
    ActionMiddleClickTitlebar(DesktopTitlebarAction),
    ColorScheme(ColorScheme),
    CursorBlink(bool),
    CursorBlinkTimeMs(i32),
    CursorBlinkTimeoutMs(i32),
    CursorSize(i32),
    CursorTheme(String),
    DoubleClickIntervalMs(i32),
    DragThresholdPixels(i32),
    EnableAnimations(bool),
    FontAntialiasing(FontAntialiasing),
    FontHinting(FontHinting),
    FontRgbaOrder(FontRgbaOrder),
    MiddleClickPaste(bool),
    OverlayScrolling(bool),
    RecentFilesEnabled(bool),
    RecentFilesMaxAgeDays(i32),
    TitlebarLayout(String),
}

impl FontAntialiasing {
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "none" => Ok(Self::None),
            "grayscale" => Ok(Self::Grayscale),
            "rgba" => Ok(Self::Rgba),
            _ => bail!("Unknown font-antialiasing value {value}"),
        }
    }
}

impl FontHinting {
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "none" => Ok(Self::None),
            "slight" => Ok(Self::Slight),
            "medium" => Ok(Self::Medium),
            "full" => Ok(Self::Full),
            _ => bail!("Unknown font-hinting value {value}"),
        }
    }
}

impl FontRgbaOrder {
    pub fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "rgb" => Ok(Self::Rgb),
            "bgr" => Ok(Self::Bgr),
            "vrgb" => Ok(Self::Vrgb),
            "vbgr" => Ok(Self::Vbgr),
            _ => bail!("Unknown font-rgba-order value {value}"),
        }
    }
}

impl DesktopTitlebarAction {
    fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "toggle-maximize" | "toggle-maximize-horizontally" | "toggle-maximize-vertically" => Ok(Self::ToggleMaximize),
            "minimize" | "lower" => Ok(Self::Minimize),
            "none" => Ok(Self::None),
            "menu" => Ok(Self::Menu),
            _ => bail!("Unknown DesktopTitlebarAction value {value}"),
        }
    }
}

impl FfiDesktopSetting<'_> {
    pub fn with<F>(s: InternalDesktopSetting, f: F)
    where
        for<'a> F: Fn(FfiDesktopSetting<'a>),
    {
        match s {
            InternalDesktopSetting::AccentColor(v) => f(Self::AccentColor(v)),
            InternalDesktopSetting::ActionDoubleClickTitlebar(v) => f(FfiDesktopSetting::ActionDoubleClickTitlebar(v)),
            InternalDesktopSetting::ActionRightClickTitlebar(v) => f(FfiDesktopSetting::ActionRightClickTitlebar(v)),
            InternalDesktopSetting::ActionMiddleClickTitlebar(v) => f(FfiDesktopSetting::ActionMiddleClickTitlebar(v)),
            InternalDesktopSetting::AudibleBell(v) => f(Self::AudibleBell(v)),
            InternalDesktopSetting::ColorScheme(v) => f(Self::ColorScheme(match v {
                ColorScheme::NoPreference => XdgDesktopColorScheme::NoPreference,
                ColorScheme::PreferDark => XdgDesktopColorScheme::PreferDark,
                ColorScheme::PreferLight => XdgDesktopColorScheme::PreferLight,
            })),
            InternalDesktopSetting::CursorBlink(v) => f(Self::CursorBlink(v)),
            InternalDesktopSetting::CursorBlinkTimeMs(v) => f(Self::CursorBlinkTimeMs(v)),
            InternalDesktopSetting::CursorBlinkTimeoutMs(v) => f(Self::CursorBlinkTimeoutMs(v)),
            InternalDesktopSetting::CursorSize(v) => f(Self::CursorSize(v)),
            InternalDesktopSetting::CursorTheme(v) => {
                f(FfiDesktopSetting::CursorTheme(BorrowedArray::new_string(&v)));
            }
            InternalDesktopSetting::EnableAnimations(v) => f(Self::EnableAnimations(v)),
            InternalDesktopSetting::DoubleClickIntervalMs(v) => f(Self::DoubleClickIntervalMs(v)),
            InternalDesktopSetting::DragThresholdPixels(v) => f(Self::DragThresholdPixels(v)),
            InternalDesktopSetting::FontAntialiasing(v) => f(Self::FontAntialiasing(v)),
            InternalDesktopSetting::FontHinting(v) => f(Self::FontHinting(v)),
            InternalDesktopSetting::FontRgbaOrder(v) => f(Self::FontRgbaOrder(v)),
            InternalDesktopSetting::MiddleClickPaste(v) => f(Self::MiddleClickPaste(v)),
            InternalDesktopSetting::OverlayScrolling(v) => f(Self::OverlayScrolling(v)),
            InternalDesktopSetting::RecentFilesEnabled(v) => f(Self::RecentFilesEnabled(v)),
            InternalDesktopSetting::RecentFilesMaxAgeDays(v) => f(Self::RecentFilesMaxAgeDays(v)),
            InternalDesktopSetting::TitlebarLayout(v) => {
                f(FfiDesktopSetting::TitlebarLayout(BorrowedArray::new_string(&v)));
            }
        }
    }
}

// dbus-send --dest=org.freedesktop.portal.Desktop --print-reply /org/freedesktop/portal/desktop org.freedesktop.portal.Settings.Read string:"org.gnome.desktop.wm.preferences" string:"button-layout"
// dbus-send --dest=org.freedesktop.portal.Desktop --print-reply /org/freedesktop/portal/desktop org.freedesktop.portal.Settings.ReadAll array:string:"org.gnome.desktop.interface","org.gnome.desktop.wm.preferences","org.freedesktop.appearance"

fn read_bool(value: &OwnedValue) -> anyhow::Result<bool> {
    value.downcast_ref::<bool>().map_err(anyhow::Error::new)
}

fn read_string(value: &OwnedValue) -> anyhow::Result<String> {
    value.downcast_ref::<String>().map_err(anyhow::Error::new)
}

fn read_i32(value: &OwnedValue) -> anyhow::Result<i32> {
    value.downcast_ref::<i32>().map_err(anyhow::Error::new)
}

//fn parse_accent_color_string(value: &str) -> Option<Color> {
//    match value {
//        "blue" => {
//
//        } "teal", "green", "yellow", "orange", "red", "pink", "purple", "slate".
//    }
//}

fn read_color(value: &OwnedValue) -> anyhow::Result<Color> {
    //    let c : ashpd::desktop::Color = value.clone().try_into()?;
    //    Ok(Color{red: c.red(), green: c.green(), blue: c.blue(), alpha: 1.0})
    let s = value.downcast_ref::<Structure>().map_err(anyhow::Error::new)?;
    let f = s.fields();
    let f1 = f.first().context("Missing first field")?;
    let f2 = f.get(1).context("Missing second field")?;
    let f3 = f.get(2).context("Missing third field")?;
    let red = f1.downcast_ref::<f64>()?;
    let green = f2.downcast_ref::<f64>()?;
    let blue = f3.downcast_ref::<f64>()?;
    Ok(Color {
        red,
        green,
        blue,
        alpha: 1.0,
    })
}

impl InternalDesktopSetting {
    pub fn new(namespace: &str, key: &str, value: &OwnedValue) -> Option<Self> {
        match Self::new_impl(namespace, key, value) {
            Ok(Some(v)) => Some(v),
            Ok(None) => {
                // debug!("Ignoring unknown setting {namespace} : {key} ({value:?})");
                None
            }
            Err(e) => {
                error!(
                    "{:?}",
                    e.context(format!("Trying to parse value {value:?} for {namespace} : {key}"))
                );
                None
            }
        }
    }

    fn new_impl(namespace: &str, key: &str, value: &OwnedValue) -> anyhow::Result<Option<Self>> {
        // We can only use entries from
        // https://github.com/GNOME/gtk/blob/2b56fd9d0e40a36ab516f49f2efc90ea7e2eacde/gdk/wayland/gdksettings-wayland.c#L267
        Ok(match namespace {
            APPEARANCE_NAMESPACE => match key {
                COLOR_SCHEME_KEY => Some(Self::ColorScheme(value.clone().try_into()?)),
                ACCENT_COLOR_SCHEME_KEY => Some(Self::AccentColor(read_color(value)?)),
                _ => None,
            },
            GNOME_DESKTOP_INTERFACE_NAMESPACE => match key {
                "cursor-blink" => Some(Self::CursorBlink(read_bool(value)?)),
                "cursor-blink-time" => Some(Self::CursorBlinkTimeMs(read_i32(value)?)),
                "cursor-blink-timeout" => Some(Self::CursorBlinkTimeoutMs(read_i32(value)? * 1000)),
                "cursor-theme" => Some(Self::CursorTheme(read_string(value)?)),
                "cursor-size" => Some(Self::CursorSize(read_i32(value)?)),
                "enable-animations" => Some(Self::EnableAnimations(read_bool(value)?)),
                "overlay-scrolling" => Some(Self::OverlayScrolling(read_bool(value)?)),
                "font-antialiasing" => Some(Self::FontAntialiasing(FontAntialiasing::parse(&read_string(value)?)?)),
                "font-hinting" => Some(Self::FontHinting(FontHinting::parse(&read_string(value)?)?)),
                "font-rgba-order" => Some(Self::FontRgbaOrder(FontRgbaOrder::parse(&read_string(value)?)?)),
                "gtk-enable-primary-paste" => Some(Self::MiddleClickPaste(read_bool(value)?)),
                _ => None,
            },
            GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE => match key {
                "double-click" => Some(Self::DoubleClickIntervalMs(read_i32(value)?)),
                "drag-threshold" => Some(Self::DragThresholdPixels(read_i32(value)?)),
                _ => None,
            },
            GNOME_DESKTOP_PRIVACY_NAMESPACE => match key {
                "recent-files-max-age" => Some(Self::RecentFilesMaxAgeDays(read_i32(value)?)),
                "remember-recent-files" => Some(Self::RecentFilesEnabled(read_bool(value)?)),
                _ => None,
            },
            GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE => match key {
                "audible-bell" => Some(Self::AudibleBell(read_bool(value)?)),
                "button-layout" => Some(Self::TitlebarLayout(read_string(value)?)),
                "action-double-click-titlebar" => {
                    Some(Self::ActionDoubleClickTitlebar(DesktopTitlebarAction::parse(&read_string(value)?)?))
                }
                "action-right-click-titlebar" => Some(Self::ActionRightClickTitlebar(DesktopTitlebarAction::parse(&read_string(value)?)?)),
                "action-middle-click-titlebar" => {
                    Some(Self::ActionMiddleClickTitlebar(DesktopTitlebarAction::parse(&read_string(value)?)?))
                }
                _ => None,
            },
            _ => None,
        })
    }
}

fn send(v: Option<InternalDesktopSetting>, sender: &dyn Fn(InternalDesktopSetting) -> anyhow::Result<()>) -> anyhow::Result<()> {
    if let Some(s) = v {
        debug!("Notifying about desktop setting: {s:?}");
        sender(s)?;
    }
    Ok(())
}

async fn read_initial_desktop_settings(
    settings: &Settings,
    sender: &(dyn Fn(InternalDesktopSetting) -> anyhow::Result<()> + Send + Sync),
) -> anyhow::Result<()> {
    let proxy = settings;

    let reply = proxy
        .call_method(
            "ReadAll",
            &[
                APPEARANCE_NAMESPACE,
                GNOME_DESKTOP_INTERFACE_NAMESPACE,
                GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE,
                GNOME_DESKTOP_PRIVACY_NAMESPACE,
                GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE,
            ]
            .as_slice(),
        )
        .await?;

    // On some systems the namespace keys are duplicated, but they have different content
    // (i.e., contain some keys that are missing in other dict entries for the same namespace).
    // That's why we deserialize to vector of tuples (to emulate the MultiMap data type).
    let all: Vec<(String, Namespace)> = reply.body().deserialize_unchecked()?;

    for (namespace, kv) in all {
        for (key, value) in kv {
            //debug!("Reading initial desktop settings from {namespace} : {key} = {value:?}");
            send(InternalDesktopSetting::new(&namespace, &key, &value), sender)?;
        }
    }
    Ok(())
}

pub async fn desktop_settings_notifier(sender: impl Fn(InternalDesktopSetting) -> anyhow::Result<()> + Send + Sync) -> anyhow::Result<()> {
    let desktop_settings = Settings::new().await?;
    read_initial_desktop_settings(&desktop_settings, &sender).await?;

    debug!("Listening to desktop settings changes");
    let mut desktop_settings_signals = desktop_settings.receive_setting_changed().await?;
    while let Some(s) = desktop_settings_signals.next().await {
        debug!("Desktop setting changed: {s:?}");
        send(InternalDesktopSetting::new(s.namespace(), s.key(), s.value()), &sender)?;
    }
    debug!("Stopped listening to desktop settings changes");
    Ok(())
}
