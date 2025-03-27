use ashpd::{desktop::settings::Settings, zvariant::OwnedValue};
use async_std::stream::StreamExt;
use desktop_common::ffi_utils::AutoDropArray;
use log::{debug, warn};
use smithay_client_toolkit::reexports::calloop::channel::Sender;

#[repr(i32)]
#[derive(Copy, Clone, Debug)]
pub enum WindowButtonType {
    AppMenu,
    Icon,
    Spacer,
    Minimize,
    Maximize,
    Close,
}

impl WindowButtonType {
    fn parse(button_name: &str) -> Option<Self> {
        match button_name {
            "appmenu" => Some(Self::AppMenu),
            "icon" => Some(Self::Icon),
            "spacer" => Some(Self::Spacer),
            "minimize" => Some(Self::Minimize),
            "maximize" => Some(Self::Maximize),
            "close" => Some(Self::Close),
            _ => {
                warn!("Unknown button name {button_name}");
                None
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct InternalTitlebarButtonLayout {
    pub left_side: Box<[WindowButtonType]>,
    pub right_side: Box<[WindowButtonType]>,
}

impl InternalTitlebarButtonLayout {
    fn parse_one_side(buttons: &str) -> Box<[WindowButtonType]> {
        buttons.split(',').filter_map(WindowButtonType::parse).collect()
    }

    fn parse(button_layout: &str) -> Self {
        let (buttons_left_str, buttons_right_str) = button_layout.split_once(':').unwrap();
        Self {
            left_side: Self::parse_one_side(buttons_left_str),
            right_side: Self::parse_one_side(buttons_right_str),
        }
    }
}

#[derive(Debug)]
pub(crate) enum InternalXdgDesktopSetting {
    TitlebarLayout(InternalTitlebarButtonLayout),
    DoubleClickIntervalMs(i32),
}

#[repr(C)]
#[derive(Debug)]
pub struct TitlebarButtonLayout {
    pub left_side: AutoDropArray<WindowButtonType>,
    pub right_side: AutoDropArray<WindowButtonType>,
}

#[repr(C)]
#[derive(Debug)]
pub enum XdgDesktopSetting {
    TitlebarLayout(TitlebarButtonLayout),
    DoubleClickIntervalMs(i32),
}

impl XdgDesktopSetting {
    pub(crate) fn with(s: InternalXdgDesktopSetting, f: impl FnOnce(Self)) {
        match s {
            InternalXdgDesktopSetting::TitlebarLayout(v) => {
                f(Self::TitlebarLayout(TitlebarButtonLayout {
                    left_side: AutoDropArray::new(v.left_side),
                    right_side: AutoDropArray::new(v.right_side),
                }));
            }
            InternalXdgDesktopSetting::DoubleClickIntervalMs(v) => f(Self::DoubleClickIntervalMs(v)),
        }
    }
}

// dbus-send --dest=org.freedesktop.portal.Desktop --print-reply /org/freedesktop/portal/desktop org.freedesktop.portal.Settings.Read string:"org.gnome.desktop.wm.preferences" string:"button-layout"
// dbus-send --dest=org.freedesktop.portal.Desktop --print-reply /org/freedesktop/portal/desktop org.freedesktop.portal.Settings.ReadAll array:string:"org.gnome.desktop.interface","org.gnome.desktop.wm.preferences","org.freedesktop.appearance"
// org.gnome.desktop.wm.preferences:
//   button-layout
//   action-double-click-titlebar
//   action-right-click-titlebar
//   action-middle-click-titlebar
//   audible-bell
// org.gnome.desktop.interface:
//   gtk-color-palette
//   cursor-blink
//   cursor-blink-time
//   overlay-scrolling
//   font-antialiasing
//   font-hinting
//   font-rgba-order
// org.freedesktop.appearance:
//   accent-color
//   color-scheme

impl InternalXdgDesktopSetting {
    #[must_use]
    pub fn new(namespace: &str, key: &str, value: &OwnedValue) -> Option<Self> {
        match namespace {
            "org.gnome.desktop.wm.preferences" => {
                match key {
                    "button-layout" => value
                        .downcast_ref::<String>()
                        .ok()
                        .map(|s| Self::TitlebarLayout(InternalTitlebarButtonLayout::parse(&s))),
                    //// Valid values: "toggle-maximize", "toggle-maximize-horizontally", "toggle-maximize-vertically", "menu", "lower", "none"
                    //"action-double-click-titlebar" => {},
                    //"action-right-click-titlebar" => {},
                    //"action-middle-click-titlebar" => {},
                    _ => None,
                }
            }
            "org.gnome.desktop.peripherals.mouse" => match key {
                "double-click" => value.downcast_ref::<i32>().ok().map(Self::DoubleClickIntervalMs),
                _ => None,
            },
            _ => None,
        }
    }
}

pub(crate) async fn xdg_desktop_settings_notifier(tx: Sender<InternalXdgDesktopSetting>) -> anyhow::Result<()> {
    let xdg_desktop_settings = Settings::new().await?;

    for (namespace, kv) in xdg_desktop_settings
        .read_all(&["org.gnome.desktop.wm.preferences", "org.gnome.desktop.peripherals.mouse"])
        .await?
    {
        for (key, value) in kv {
            if let Some(s) = InternalXdgDesktopSetting::new(&namespace, &key, &value) {
                debug!("xdg_desktop_settings_notifier: {s:?}");
                tx.send(s)?;
            }
        }
    }

    let mut xdg_desktop_settings_signals = xdg_desktop_settings.receive_setting_changed().await?;
    while let Some(s) = xdg_desktop_settings_signals.next().await {
        debug!("xdg_desktop_settings_notifier: {s:?}");
        if let Some(e) = InternalXdgDesktopSetting::new(s.namespace(), s.key(), s.value()) {
            tx.send(e)?;
        }
    }
    //    Ok(xdg_desktop_settings_signals.filter_map(async move |s| XdgDesktopSetting::new(s.namespace(), s.key(), s.value())))
    debug!("xdg_desktop_settings_notifier stopping");
    Ok(())
}
