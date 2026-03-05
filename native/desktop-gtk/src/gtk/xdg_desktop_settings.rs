use std::ffi::CString;

use crate::gtk::application_state::with_default_display;
use crate::gtk::xdg_desktop_settings_api::{
    Color, DesktopTitlebarAction, FontAntialiasing, FontHinting, FontRgbaOrder, XdgDesktopColorScheme, XdgDesktopSetting,
};
use anyhow::bail;
use desktop_common::ffi_utils::BorrowedStrPtr;
use gtk4::prelude::{SettingsExt, SettingsExtManual};
use gtk4::{gdk as gdk4, gio, glib};

#[derive(Debug)]
pub enum InternalXdgDesktopSetting {
    TitlebarLayout(String),

    ActionDoubleClickTitlebar(DesktopTitlebarAction),
    ActionRightClickTitlebar(DesktopTitlebarAction),
    ActionMiddleClickTitlebar(DesktopTitlebarAction),

    DoubleClickIntervalMs(i32),
    ColorScheme(XdgDesktopColorScheme),
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

    MiddleClickPaste(bool),
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
    pub fn parse(value: &glib::GString) -> anyhow::Result<Self> {
        match value.as_str() {
            "hintnone" => Ok(Self::None),
            "hintslight" => Ok(Self::Slight),
            "hintmedium" => Ok(Self::Medium),
            "hintfull" => Ok(Self::Full),
            _ => bail!("Unknown font-hinting value {value}"),
        }
    }
}

impl FontRgbaOrder {
    pub fn parse(value: &glib::GString) -> anyhow::Result<Self> {
        match value.as_str() {
            "none" => Ok(Self::None),
            "rgb" => Ok(Self::Rgb),
            "bgr" => Ok(Self::Bgr),
            "vrgb" => Ok(Self::Vrgb),
            "vbgr" => Ok(Self::Vbgr),
            _ => bail!("Unknown font-rgba-order value {value}"),
        }
    }
}

impl DesktopTitlebarAction {
    fn parse(value: &glib::GString) -> anyhow::Result<Self> {
        match value.as_str() {
            "toggle-maximize" | "toggle-maximize-horizontally" | "toggle-maximize-vertically" => Ok(Self::ToggleMaximize),
            "minimize" | "lower" => Ok(Self::Minimize),
            "none" => Ok(Self::None),
            "menu" => Ok(Self::Menu),
            _ => bail!("Unknown DesktopTitlebarAction value {value}"),
        }
    }
}

impl XdgDesktopColorScheme {
    fn parse(value: &str) -> anyhow::Result<Self> {
        match value {
            "default" => Ok(Self::NoPreference),
            "prefer-dark" => Ok(Self::PreferDark),
            "prefer-light" => Ok(Self::PreferLight),
            _ => bail!("Unknown XdgDesktopColorScheme value {value}"),
        }
    }
}

impl XdgDesktopSetting<'_> {
    pub fn with<F>(s: InternalXdgDesktopSetting, f: F)
    where
        for<'a> F: Fn(XdgDesktopSetting<'a>),
    {
        match s {
            InternalXdgDesktopSetting::TitlebarLayout(v) => {
                let cs = CString::new(v).unwrap();
                f(XdgDesktopSetting::TitlebarLayout(BorrowedStrPtr::new(&cs)));
            }
            InternalXdgDesktopSetting::ActionDoubleClickTitlebar(v) => f(XdgDesktopSetting::ActionDoubleClickTitlebar(v)),
            InternalXdgDesktopSetting::ActionRightClickTitlebar(v) => f(XdgDesktopSetting::ActionRightClickTitlebar(v)),
            InternalXdgDesktopSetting::ActionMiddleClickTitlebar(v) => f(XdgDesktopSetting::ActionMiddleClickTitlebar(v)),
            InternalXdgDesktopSetting::DoubleClickIntervalMs(v) => f(Self::DoubleClickIntervalMs(v)),
            InternalXdgDesktopSetting::ColorScheme(v) => f(Self::ColorScheme(v)),
            InternalXdgDesktopSetting::AccentColor(v) => f(Self::AccentColor(v)),
            InternalXdgDesktopSetting::FontAntialiasing(v) => f(Self::FontAntialiasing(v)),
            InternalXdgDesktopSetting::FontHinting(v) => f(Self::FontHinting(v)),
            InternalXdgDesktopSetting::FontRgbaOrder(v) => f(Self::FontRgbaOrder(v)),
            InternalXdgDesktopSetting::CursorBlink(v) => f(Self::CursorBlink(v)),
            InternalXdgDesktopSetting::CursorBlinkTimeMs(v) => f(Self::CursorBlinkTimeMs(v)),
            InternalXdgDesktopSetting::CursorBlinkTimeoutMs(v) => f(Self::CursorBlinkTimeoutMs(v)),
            InternalXdgDesktopSetting::OverlayScrolling(v) => f(Self::OverlayScrolling(v)),
            InternalXdgDesktopSetting::AudibleBell(v) => f(Self::AudibleBell(v)),
            InternalXdgDesktopSetting::CursorSize(v) => f(Self::CursorSize(v)),
            InternalXdgDesktopSetting::CursorTheme(v) => {
                let cs = CString::new(v).unwrap();
                f(XdgDesktopSetting::CursorTheme(BorrowedStrPtr::new(&cs)));
            }
            InternalXdgDesktopSetting::MiddleClickPaste(v) => f(Self::MiddleClickPaste(v)),
        }
    }
}

pub struct DesktopSettings {
    settings: gtk4::Settings,
    gio_schema_and_settings: Option<(gio::SettingsSchema, gio::Settings)>,
}

impl DesktopSettings {
    pub fn new(display_manager: &gdk4::DisplayManager) -> Self {
        let settings = with_default_display(display_manager, gtk4::Settings::for_display);
        let gio_schema_and_settings = gio::SettingsSchemaSource::default()
            .and_then(|source| source.lookup("org.gnome.desktop.interface", true))
            .map(|schema| (schema, gio::Settings::new("org.gnome.desktop.interface")));
        Self {
            settings,
            gio_schema_and_settings,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn read_and_subscribe(&self, sender: impl Fn(InternalXdgDesktopSetting) + Clone + 'static) -> anyhow::Result<()> {
        if let Some(value) = self.settings.gtk_decoration_layout() {
            sender(InternalXdgDesktopSetting::TitlebarLayout(value.as_str().to_owned()));
            let sender = sender.clone();
            self.settings.connect_gtk_decoration_layout_notify(move |s| {
                if let Some(value) = s.gtk_decoration_layout() {
                    sender(InternalXdgDesktopSetting::TitlebarLayout(value.as_str().to_owned()));
                }
            });
        }

        if let Some(value) = self.settings.gtk_titlebar_double_click() {
            let action = DesktopTitlebarAction::parse(&value)?;
            sender(InternalXdgDesktopSetting::ActionDoubleClickTitlebar(action));
            let sender = sender.clone();
            self.settings.connect_gtk_titlebar_double_click_notify(move |s| {
                if let Some(value) = s.gtk_titlebar_double_click() {
                    let action = DesktopTitlebarAction::parse(&value).unwrap();
                    sender(InternalXdgDesktopSetting::ActionDoubleClickTitlebar(action));
                }
            });
        }

        if let Some(value) = self.settings.gtk_titlebar_right_click() {
            let action = DesktopTitlebarAction::parse(&value)?;
            sender(InternalXdgDesktopSetting::ActionRightClickTitlebar(action));
            let sender = sender.clone();
            self.settings.connect_gtk_titlebar_right_click_notify(move |s| {
                if let Some(value) = s.gtk_titlebar_right_click() {
                    let action = DesktopTitlebarAction::parse(&value).unwrap();
                    sender(InternalXdgDesktopSetting::ActionRightClickTitlebar(action));
                }
            });
        }

        if let Some(value) = self.settings.gtk_titlebar_middle_click() {
            let action = DesktopTitlebarAction::parse(&value)?;
            sender(InternalXdgDesktopSetting::ActionMiddleClickTitlebar(action));
            let sender = sender.clone();
            self.settings.connect_gtk_titlebar_middle_click_notify(move |s| {
                if let Some(value) = s.gtk_titlebar_middle_click() {
                    let action = DesktopTitlebarAction::parse(&value).unwrap();
                    sender(InternalXdgDesktopSetting::ActionMiddleClickTitlebar(action));
                }
            });
        }

        sender(InternalXdgDesktopSetting::DoubleClickIntervalMs(
            self.settings.gtk_double_click_time(),
        ));
        {
            let sender = sender.clone();
            self.settings.connect_gtk_double_click_time_notify(move |s| {
                sender(InternalXdgDesktopSetting::DoubleClickIntervalMs(s.gtk_double_click_time()));
            });
        }

        if let Some(value) = self.settings.gtk_xft_hintstyle() {
            sender(InternalXdgDesktopSetting::FontHinting(FontHinting::parse(&value)?));
            {
                let sender = sender.clone();
                self.settings.connect_gtk_xft_hintstyle_notify(move |s| {
                    if let Some(value) = s.gtk_xft_hintstyle() {
                        let hinting = FontHinting::parse(&value).unwrap();
                        sender(InternalXdgDesktopSetting::FontHinting(hinting));
                    }
                });
            }
        }

        let rgba_order = if let Some(value) = self.settings.gtk_xft_rgba() {
            let rgba_order = FontRgbaOrder::parse(&value)?;
            sender(InternalXdgDesktopSetting::FontRgbaOrder(rgba_order));
            {
                let sender = sender.clone();
                self.settings.connect_gtk_xft_rgba_notify(move |s| {
                    if let Some(value) = s.gtk_xft_rgba() {
                        let rgba_order = FontRgbaOrder::parse(&value).unwrap();
                        sender(InternalXdgDesktopSetting::FontRgbaOrder(rgba_order));
                        // TODO: also notify for FontAntialiasing
                    }
                });
            }
            Some(rgba_order)
        } else {
            None
        };

        let antialias = self.settings.gtk_xft_antialias();
        let font_aliasing = if antialias == 0 {
            FontAntialiasing::None
        } else if rgba_order == Some(FontRgbaOrder::None) {
            FontAntialiasing::Grayscale
        } else {
            FontAntialiasing::Rgba
        };
        sender(InternalXdgDesktopSetting::FontAntialiasing(font_aliasing));
        // TODO: add notify for FontAntialiasing

        sender(InternalXdgDesktopSetting::CursorBlink(self.settings.is_gtk_cursor_blink()));
        {
            let sender = sender.clone();
            self.settings.connect_gtk_cursor_blink_notify(move |s| {
                sender(InternalXdgDesktopSetting::CursorBlink(s.is_gtk_cursor_blink()));
            });
        }

        sender(InternalXdgDesktopSetting::CursorSize(self.settings.gtk_cursor_theme_size()));
        {
            let sender = sender.clone();
            self.settings.connect_gtk_cursor_theme_size_notify(move |s| {
                sender(InternalXdgDesktopSetting::CursorSize(s.gtk_cursor_theme_size()));
            });
        }

        if let Some(value) = self.settings.gtk_cursor_theme_name() {
            sender(InternalXdgDesktopSetting::CursorTheme(value.as_str().to_owned()));
        }
        {
            let sender = sender.clone();
            self.settings.connect_gtk_cursor_theme_name_notify(move |s| {
                if let Some(value) = s.gtk_cursor_theme_name() {
                    sender(InternalXdgDesktopSetting::CursorTheme(value.as_str().to_owned()));
                }
            });
        }

        sender(InternalXdgDesktopSetting::CursorBlinkTimeMs(self.settings.gtk_cursor_blink_time()));
        {
            let sender = sender.clone();
            self.settings.connect_gtk_cursor_blink_time_notify(move |s| {
                sender(InternalXdgDesktopSetting::CursorBlinkTimeMs(s.gtk_cursor_blink_time()));
            });
        }

        sender(InternalXdgDesktopSetting::CursorBlinkTimeoutMs(
            self.settings.gtk_cursor_blink_timeout(),
        ));
        {
            let sender = sender.clone();
            self.settings.connect_gtk_cursor_blink_timeout_notify(move |s| {
                sender(InternalXdgDesktopSetting::CursorBlinkTimeoutMs(s.gtk_cursor_blink_timeout()));
            });
        }

        sender(InternalXdgDesktopSetting::OverlayScrolling(
            self.settings.is_gtk_overlay_scrolling(),
        ));
        {
            let sender = sender.clone();
            self.settings.connect_gtk_overlay_scrolling_notify(move |s| {
                sender(InternalXdgDesktopSetting::OverlayScrolling(s.is_gtk_overlay_scrolling()));
            });
        }

        sender(InternalXdgDesktopSetting::AudibleBell(self.settings.is_gtk_error_bell()));
        {
            let sender = sender.clone();
            self.settings.connect_gtk_error_bell_notify(move |s| {
                sender(InternalXdgDesktopSetting::AudibleBell(s.is_gtk_error_bell()));
            });
        }

        sender(InternalXdgDesktopSetting::MiddleClickPaste(
            self.settings.is_gtk_enable_primary_paste(),
        ));
        {
            let sender = sender.clone();
            self.settings.connect_gtk_enable_primary_paste_notify(move |s| {
                sender(InternalXdgDesktopSetting::MiddleClickPaste(s.is_gtk_enable_primary_paste()));
            });
        }

        if let Some((gio_schema, gio_settings)) = &self.gio_schema_and_settings {
            if gio_schema.has_key("color-scheme") {
                {
                    let sender = sender.clone();
                    gio_settings.connect_changed(Some("color-scheme"), move |s, name| {
                        dbg!(name);
                        let color_theme: String = s.get("color-scheme");
                        dbg!(&color_theme);
                        sender(InternalXdgDesktopSetting::ColorScheme(
                            XdgDesktopColorScheme::parse(&color_theme).unwrap(),
                        ));
                    });
                }
                let color_theme: String = gio_settings.get("color-scheme");
                sender(InternalXdgDesktopSetting::ColorScheme(XdgDesktopColorScheme::parse(&color_theme)?));
                dbg!(color_theme);
            }
            if gio_schema.has_key("accent-color") {
                gio_settings.connect_changed(Some("accent-color"), |s, name| {
                    dbg!(name);
                    let accent_color: String = s.get("accent-color");
                    dbg!(accent_color);
                });
                let accent_color: String = gio_settings.get("accent-color");
                dbg!(accent_color);
            }
        }
        Ok(())
    }
}
