use crate::linux::desktop_settings_api::{
    Color, DesktopTitlebarAction, FfiDesktopSetting, FontAntialiasing, FontHinting, FontRgbaOrder, XdgDesktopColorScheme,
};
use anyhow::bail;
use ashpd::{
    desktop::settings::{ACCENT_COLOR_SCHEME_KEY, APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY, ColorScheme, Namespace},
    zvariant::OwnedValue,
};
use desktop_common::ffi_utils::BorrowedArray;
use futures_lite::StreamExt;
use log::{debug, error, warn};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

/// cbindgen:ignore
const GNOME_DESKTOP_INTERFACE_NAMESPACE: &str = "org.gnome.desktop.interface";

/// cbindgen:ignore
const GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE: &str = "org.gnome.desktop.peripherals.mouse";

/// cbindgen:ignore
const GNOME_DESKTOP_PRIVACY_NAMESPACE: &str = "org.gnome.desktop.privacy";

/// cbindgen:ignore
const GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE: &str = "org.gnome.desktop.wm.preferences";

#[derive(Clone, Debug, PartialEq)]
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
    pub fn parse(value: &String) -> anyhow::Result<Self> {
        match value.as_str() {
            "none" => Ok(Self::None),
            "grayscale" => Ok(Self::Grayscale),
            "rgba" => Ok(Self::Rgba),
            _ => bail!("Unknown font-antialiasing value {value}"),
        }
    }
}

impl FontHinting {
    pub fn parse(value: &String) -> anyhow::Result<Self> {
        match value.as_str() {
            "none" => Ok(Self::None),
            "slight" => Ok(Self::Slight),
            "medium" => Ok(Self::Medium),
            "full" => Ok(Self::Full),
            _ => bail!("Unknown font-hinting value {value}"),
        }
    }
}

impl FontRgbaOrder {
    pub fn parse(value: &String) -> anyhow::Result<Self> {
        match value.as_str() {
            "rgb" => Ok(Self::Rgb),
            "bgr" => Ok(Self::Bgr),
            "vrgb" => Ok(Self::Vrgb),
            "vbgr" => Ok(Self::Vbgr),
            _ => bail!("Unknown font-rgba-order value {value}"),
        }
    }
}

impl DesktopTitlebarAction {
    fn parse(value: &String) -> anyhow::Result<Self> {
        match value.as_str() {
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

fn read_color(value: OwnedValue) -> anyhow::Result<Color> {
    let (red, green, blue): (f64, f64, f64) = value.try_into()?;
    Ok(Color {
        red,
        green,
        blue,
        alpha: 1.0,
    })
}

impl InternalDesktopSetting {
    pub fn new(namespace: &str, key: &str, value: OwnedValue) -> Option<Self> {
        match Self::new_impl(namespace, key, value) {
            Ok(Some(v)) => Some(v),
            Ok(None) => None,
            Err(e) => {
                error!("{:?}", e.context(format!("Trying to parse value for {namespace} : {key}")));
                None
            }
        }
    }

    fn new_impl(namespace: &str, key: &str, value: OwnedValue) -> anyhow::Result<Option<Self>> {
        // We can only use entries from
        // https://github.com/GNOME/gtk/blob/2b56fd9d0e40a36ab516f49f2efc90ea7e2eacde/gdk/wayland/gdksettings-wayland.c#L267
        Ok(match namespace {
            APPEARANCE_NAMESPACE => match key {
                COLOR_SCHEME_KEY => Some(Self::ColorScheme(value.try_into()?)),
                ACCENT_COLOR_SCHEME_KEY => Some(Self::AccentColor(read_color(value)?)),
                _ => None,
            },
            GNOME_DESKTOP_INTERFACE_NAMESPACE => match key {
                "cursor-blink" => Some(Self::CursorBlink(value.try_into()?)),
                "cursor-blink-time" => Some(Self::CursorBlinkTimeMs(value.try_into()?)),
                "cursor-blink-timeout" => Some(Self::CursorBlinkTimeoutMs(TryInto::<i32>::try_into(value)? * 1000)),
                "cursor-theme" => Some(Self::CursorTheme(value.try_into()?)),
                "cursor-size" => Some(Self::CursorSize(value.try_into()?)),
                "enable-animations" => Some(Self::EnableAnimations(value.try_into()?)),
                "overlay-scrolling" => Some(Self::OverlayScrolling(value.try_into()?)),
                "font-antialiasing" => Some(Self::FontAntialiasing(FontAntialiasing::parse(&value.try_into()?)?)),
                "font-hinting" => Some(Self::FontHinting(FontHinting::parse(&value.try_into()?)?)),
                "font-rgba-order" => Some(Self::FontRgbaOrder(FontRgbaOrder::parse(&value.try_into()?)?)),
                "gtk-enable-primary-paste" => Some(Self::MiddleClickPaste(value.try_into()?)),
                _ => None,
            },
            GNOME_DESKTOP_PERIPHERALS_MOUSE_NAMESPACE => match key {
                "double-click" => Some(Self::DoubleClickIntervalMs(value.try_into()?)),
                "drag-threshold" => Some(Self::DragThresholdPixels(value.try_into()?)),
                _ => None,
            },
            GNOME_DESKTOP_PRIVACY_NAMESPACE => match key {
                "recent-files-max-age" => Some(Self::RecentFilesMaxAgeDays(value.try_into()?)),
                "remember-recent-files" => Some(Self::RecentFilesEnabled(value.try_into()?)),
                _ => None,
            },
            GNOME_DESKTOP_WM_PREFERENCES_NAMESPACE => match key {
                "audible-bell" => Some(Self::AudibleBell(value.try_into()?)),
                "button-layout" => Some(Self::TitlebarLayout(value.try_into()?)),
                "action-double-click-titlebar" => Some(Self::ActionDoubleClickTitlebar(DesktopTitlebarAction::parse(&value.try_into()?)?)),
                "action-right-click-titlebar" => Some(Self::ActionRightClickTitlebar(DesktopTitlebarAction::parse(&value.try_into()?)?)),
                "action-middle-click-titlebar" => Some(Self::ActionMiddleClickTitlebar(DesktopTitlebarAction::parse(&value.try_into()?)?)),
                _ => None,
            },
            _ => None,
        })
    }
}

fn desktop_settings_from_namespace(namespace: &str, values: Namespace) -> HashMap<String, InternalDesktopSetting> {
    values
        .into_iter()
        .filter_map(|(key, value)| InternalDesktopSetting::new(namespace, &key, value).map(|s| (key, s)))
        .collect()
}

async fn read_initial_desktop_settings(
    settings: &zbus::proxy::Proxy<'_>,
) -> anyhow::Result<HashMap<String, HashMap<String, InternalDesktopSetting>>> {
    let reply = settings
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
    let all_maybe_duplicated: Vec<(String, Namespace)> = reply.body().deserialize_unchecked()?;

    let mut all = HashMap::<String, HashMap<String, InternalDesktopSetting>>::new();

    for (namespace, kv) in all_maybe_duplicated {
        match all.entry(namespace) {
            Entry::Occupied(occupied) => {
                let settings = desktop_settings_from_namespace(occupied.key(), kv);
                let occupied = occupied.into_mut();
                for (new_key, new_setting) in settings {
                    match occupied.entry(new_key) {
                        Entry::Occupied(occupied_nested) => {
                            let occupied_value = occupied_nested.into_mut();
                            if *occupied_value != new_setting {
                                warn!("Overwriting existing setting: {occupied_value:?} with {new_setting:?}");
                                *occupied_value = new_setting;
                            }
                        }
                        Entry::Vacant(vacant) => {
                            vacant.insert(new_setting);
                        }
                    }
                }
            }
            Entry::Vacant(vacant) => {
                let settings = desktop_settings_from_namespace(vacant.key(), kv);
                vacant.insert(settings);
            }
        }
    }

    Ok(all)
}

async fn desktop_settings_notifier(
    desktop_settings: zbus::proxy::Proxy<'_>,
    sender: impl Fn(InternalDesktopSetting) -> anyhow::Result<()> + Send + Sync,
) -> anyhow::Result<()> {
    let mut desktop_settings_signals = desktop_settings.receive_signal("SettingChanged").await?;
    debug!("Created desktop settings signal stream");

    let mut all = read_initial_desktop_settings(&desktop_settings).await?;
    debug!("Received initial desktop settings");

    for namespace in all.values() {
        for value in namespace.values() {
            //debug!("Reading initial desktop settings from {namespace} : {key} = {value:?}");
            sender(value.clone())?;
        }
    }

    debug!("Listening to desktop settings changes");
    while let Some(change) = desktop_settings_signals.next().await {
        let (change_namespace, change_key, change_value): (String, String, OwnedValue) = change.body().deserialize()?;
        debug!("Desktop setting changed: {change_namespace} : {change_key} = {change_value:?}");
        if let Some(ns) = all.get_mut(&change_namespace)
            && let Some(prev) = ns.get_mut(&change_key)
            && let Some(s) = InternalDesktopSetting::new(&change_namespace, &change_key, change_value)
            && s != *prev
        {
            sender(s.clone())?;
            *prev = s;
        }
    }
    debug!("Stopped listening to desktop settings changes");
    Ok(())
}

pub async fn init_desktop_settings_notifier_task(
    sender: impl Fn(InternalDesktopSetting) -> anyhow::Result<()> + Send + Sync + 'static,
) -> anyhow::Result<()> {
    let connection = zbus::Connection::session().await?;
    debug!("Created desktop settings connection");

    // Don't use ashpd Settings, because it uses a static connection instance,
    // which causes an issue when application is reinitialized (e.g. in tests).
    // https://github.com/bilelmoussaoui/ashpd/blob/c91fe03807b9a72b56f57acd2f3efb53f0132560/client/src/proxy.rs#L27
    let desktop_settings = zbus::proxy::Builder::new(&connection)
        .interface("org.freedesktop.portal.Settings")?
        .path("/org/freedesktop/portal/desktop")?
        .destination("org.freedesktop.portal.Desktop")?
        .build()
        .await?;
    debug!("Created desktop settings proxy");

    desktop_settings_notifier(desktop_settings, sender).await
}
