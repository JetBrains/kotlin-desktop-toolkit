use std::ffi::CString;

use anyhow::{Context, bail};
use ashpd::{
    desktop::settings::{ACCENT_COLOR_SCHEME_KEY, APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY, ColorScheme, Namespace, Settings},
    zvariant::{OwnedValue, Structure},
};
use async_std::stream::StreamExt;
use desktop_common::ffi_utils::BorrowedStrPtr;
use log::{debug, error};
use smithay_client_toolkit::reexports::calloop::channel::Sender;

use crate::linux::xdg_desktop_settings_api::{
    Color, FontAntialiasing, FontHinting, FontRgbaOrder, XdgDesktopColorScheme, XdgDesktopSetting,
};

const GNOME_DESKTOP_INTERFACE_NAMESPACE: &str = "org.gnome.desktop.interface";
const GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE: &str = "org.gnome.desktop.peripherals.mouse";
const GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE: &str = "org.gnome.desktop.wm.preferences";

#[derive(Debug)]
pub enum InternalXdgDesktopSetting {
    TitlebarLayout(String),
    DoubleClickIntervalMs(i32),
    ColorScheme(ColorScheme),
    AccentColor(Color),
    FontAntialiasing(FontAntialiasing),
    FontHinting(FontHinting),
    FontRgbaOrder(FontRgbaOrder),

    CursorBlink(bool),
    CursorSize(i32),
    CursorTheme(String),

    /// Length of the cursor blink cycle, in milliseconds.
    CursorBlinkTimeMs(i32),

    /// Time after which the cursor stops blinking.
    CursorBlinkTimeoutMs(i32),

    OverlayScrolling(bool),

    AudibleBell(bool),
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

impl XdgDesktopSetting<'_> {
    pub fn with(s: InternalXdgDesktopSetting, f: impl FnOnce(&XdgDesktopSetting)) {
        match s {
            InternalXdgDesktopSetting::TitlebarLayout(v) => {
                let cs = CString::new(v).unwrap();
                f(&XdgDesktopSetting::TitlebarLayout(BorrowedStrPtr::new(&cs)));
            }
            InternalXdgDesktopSetting::DoubleClickIntervalMs(v) => f(&Self::DoubleClickIntervalMs(v)),
            InternalXdgDesktopSetting::ColorScheme(v) => f(&Self::ColorScheme(match v {
                ColorScheme::NoPreference => XdgDesktopColorScheme::NoPreference,
                ColorScheme::PreferDark => XdgDesktopColorScheme::PreferDark,
                ColorScheme::PreferLight => XdgDesktopColorScheme::PreferLight,
            })),
            InternalXdgDesktopSetting::AccentColor(v) => f(&Self::AccentColor(v)),
            InternalXdgDesktopSetting::FontAntialiasing(v) => f(&Self::FontAntialiasing(v)),
            InternalXdgDesktopSetting::FontHinting(v) => f(&Self::FontHinting(v)),
            InternalXdgDesktopSetting::FontRgbaOrder(v) => f(&Self::FontRgbaOrder(v)),
            InternalXdgDesktopSetting::CursorBlink(v) => f(&Self::CursorBlink(v)),
            InternalXdgDesktopSetting::CursorBlinkTimeMs(v) => f(&Self::CursorBlinkTimeMs(v)),
            InternalXdgDesktopSetting::CursorBlinkTimeoutMs(v) => f(&Self::CursorBlinkTimeoutMs(v)),
            InternalXdgDesktopSetting::OverlayScrolling(v) => f(&Self::OverlayScrolling(v)),
            InternalXdgDesktopSetting::AudibleBell(v) => f(&Self::AudibleBell(v)),
            InternalXdgDesktopSetting::CursorSize(v) => f(&Self::CursorSize(v)),
            InternalXdgDesktopSetting::CursorTheme(v) => {
                let cs = CString::new(v).unwrap();
                f(&XdgDesktopSetting::CursorTheme(BorrowedStrPtr::new(&cs)));
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
    let s = value.downcast_ref::<Structure>().map_err(anyhow::Error::new).unwrap();
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

impl InternalXdgDesktopSetting {
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
                "overlay-scrolling" => Some(Self::OverlayScrolling(read_bool(value)?)),
                "font-antialiasing" => Some(Self::FontAntialiasing(FontAntialiasing::parse(&read_string(value)?)?)),
                "font-hinting" => Some(Self::FontHinting(FontHinting::parse(&read_string(value)?)?)),
                "font-rgba-order" => Some(Self::FontRgbaOrder(FontRgbaOrder::parse(&read_string(value)?)?)),
                _ => None,
            },
            GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE => match key {
                "double-click" => Some(Self::DoubleClickIntervalMs(read_i32(value)?)),
                _ => None,
            },
            GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE => {
                match key {
                    "audible-bell" => Some(Self::AudibleBell(read_bool(value)?)),
                    "button-layout" => Some(Self::TitlebarLayout(read_string(value)?)),
                    //// Valid values: "toggle-maximize", "toggle-maximize-horizontally", "toggle-maximize-vertically", "menu", "lower", "none"
                    //"action-double-click-titlebar" => {},
                    //"action-right-click-titlebar" => {},
                    //"action-middle-click-titlebar" => {},
                    _ => None,
                }
            }
            _ => None,
        })
    }
}

fn send(v: Option<InternalXdgDesktopSetting>, tx: &Sender<InternalXdgDesktopSetting>) -> anyhow::Result<()> {
    if let Some(s) = v {
        debug!("Notifying about XDG setting: {s:?}");
        tx.send(s)?;
    }
    Ok(())
}

async fn read_initial_xdg_desktop_settings(settings: &Settings<'_>, tx: &Sender<InternalXdgDesktopSetting>) -> anyhow::Result<()> {
    let proxy = settings;

    let reply = proxy
        .call_method(
            "ReadAll",
            &[
                APPEARANCE_NAMESPACE,
                GNOME_DESKTOP_INTERFACE_NAMESPACE,
                GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE,
                GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE,
            ]
            .as_slice(),
        )
        .await
        .unwrap();

    // On some systems the namespace keys are duplicated, but they have different content
    // (i.e., contain some keys that are missing in other dict entries for the same namespace).
    // That's why we deserialize to vector of tuples (to emulate the MultiMap data type).
    let all: Vec<(String, Namespace)> = reply.body().deserialize_unchecked()?;

    for (namespace, kv) in all {
        for (key, value) in kv {
            //debug!("Reading initial XDG settings from {namespace} : {key} = {value:?}");
            send(InternalXdgDesktopSetting::new(&namespace, &key, &value), tx)?;
        }
    }
    Ok(())
}

pub async fn xdg_desktop_settings_notifier(tx: Sender<InternalXdgDesktopSetting>) -> anyhow::Result<()> {
    // let d = ashpd::desktop::remote_desktop::RemoteDesktop::new().await?;
    // let c = ashpd::desktop::clipboard::Clipboard::new().await?;
    let xdg_desktop_settings = Settings::new().await?;
    read_initial_xdg_desktop_settings(&xdg_desktop_settings, &tx).await?;

    debug!("Listening to XDG settings changes");
    let mut xdg_desktop_settings_signals = xdg_desktop_settings.receive_setting_changed().await?;
    while let Some(s) = xdg_desktop_settings_signals.next().await {
        debug!("XDG setting changed: {s:?}");
        send(InternalXdgDesktopSetting::new(s.namespace(), s.key(), s.value()), &tx)?;
    }
    debug!("Stopped listening to XDG settings changes");
    Ok(())
}
