use crate::gtk::desktop_settings_api::{AccentColor, FfiDesktopSetting, FontHinting, FontRgbaOrder, XdgDesktopColorScheme};
use anyhow::bail;
use gtk4::prelude::{DisplayExt, ObjectExt, SettingsExt, SettingsExtManual};
use gtk4::{gdk as gdk4, gio, glib};
use log::debug;

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

impl XdgDesktopColorScheme {
    fn parse(value: &String) -> anyhow::Result<Self> {
        match value.as_str() {
            "default" => Ok(Self::NoPreference),
            "prefer-dark" => Ok(Self::PreferDark),
            "prefer-light" => Ok(Self::PreferLight),
            _ => bail!("Unknown XdgDesktopColorScheme value {value}"),
        }
    }
}

impl AccentColor {
    fn parse(value: &String) -> anyhow::Result<Self> {
        match value.as_str() {
            "blue" => Ok(Self::Blue),
            "teal" => Ok(Self::Teal),
            "green" => Ok(Self::Green),
            "yellow" => Ok(Self::Yellow),
            "orange" => Ok(Self::Orange),
            "red" => Ok(Self::Red),
            "pink" => Ok(Self::Pink),
            "purple" => Ok(Self::Purple),
            "slate" => Ok(Self::Slate),
            _ => bail!("Unknown accent color value {value}"),
        }
    }
}

fn read_font_rgba_order(s: &gtk4::Settings) -> anyhow::Result<FontRgbaOrder> {
    let value = s.gtk_xft_rgba();
    Ok(value.as_ref().map(FontRgbaOrder::parse).transpose()?.unwrap_or(FontRgbaOrder::None))
}

struct AppSettings {
    gtk_settings: gtk4::Settings,
    settings_handlers: Vec<glib::SignalHandlerId>,
    initial_settings: Vec<FfiDesktopSetting>,
}

impl Drop for AppSettings {
    fn drop(&mut self) {
        for handler_id in std::mem::take(&mut self.settings_handlers) {
            self.gtk_settings.disconnect(handler_id);
        }
    }
}

impl AppSettings {
    #[allow(clippy::type_complexity)]
    fn read_and_subscribe_single(
        &mut self,
        sender: impl Fn(FfiDesktopSetting) + Copy + 'static,
        subscribe: fn(&gtk4::Settings, Box<dyn Fn(&gtk4::Settings)>) -> glib::SignalHandlerId,
        r: impl Fn(&gtk4::Settings) -> anyhow::Result<FfiDesktopSetting> + 'static,
    ) -> anyhow::Result<()> {
        self.initial_settings.push(r(&self.gtk_settings)?);
        let handler_id = subscribe(
            &self.gtk_settings,
            Box::new(move |s| {
                let v = r(s).unwrap();
                sender(v);
            }),
        );
        self.settings_handlers.push(handler_id);
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub fn read_and_subscribe(&mut self, sender: impl Fn(FfiDesktopSetting) + Copy + 'static) -> anyhow::Result<()> {
        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_double_click_time_notify, move |s| {
            Ok(FfiDesktopSetting::DoubleClickIntervalMs(s.gtk_double_click_time()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_xft_hintstyle_notify, move |s| {
            let value = s.gtk_xft_hintstyle();
            let hinting = value.as_ref().map(FontHinting::parse).transpose()?.unwrap_or(FontHinting::None);
            Ok(FfiDesktopSetting::FontHinting(hinting))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_xft_rgba_notify, move |s| {
            Ok(FfiDesktopSetting::FontRgbaOrder(read_font_rgba_order(s)?))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_cursor_blink_notify, move |s| {
            Ok(FfiDesktopSetting::CursorBlink(s.is_gtk_cursor_blink()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_cursor_blink_time_notify, move |s| {
            Ok(FfiDesktopSetting::CursorBlinkTimeMs(s.gtk_cursor_blink_time()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_cursor_blink_timeout_notify, move |s| {
            Ok(FfiDesktopSetting::CursorBlinkTimeoutSeconds(s.gtk_cursor_blink_timeout()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_overlay_scrolling_notify, move |s| {
            Ok(FfiDesktopSetting::OverlayScrolling(s.is_gtk_overlay_scrolling()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_enable_event_sounds_notify, move |s| {
            Ok(FfiDesktopSetting::AudibleBell(s.is_gtk_enable_event_sounds()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_enable_primary_paste_notify, move |s| {
            Ok(FfiDesktopSetting::MiddleClickPaste(s.is_gtk_enable_primary_paste()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_dnd_drag_threshold_notify, move |s| {
            Ok(FfiDesktopSetting::DragAndDropDragThresholdPixels(s.gtk_dnd_drag_threshold()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_double_click_distance_notify, move |s| {
            Ok(FfiDesktopSetting::DoubleClickDistancePixels(s.gtk_double_click_distance()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_enable_animations_notify, move |s| {
            Ok(FfiDesktopSetting::EnableAnimations(s.is_gtk_enable_animations()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_primary_button_warps_slider_notify, move |s| {
            Ok(FfiDesktopSetting::PrimaryButtonWarpsSlider(s.is_gtk_primary_button_warps_slider()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_recent_files_enabled_notify, move |s| {
            Ok(FfiDesktopSetting::RecentFilesEnabled(s.is_gtk_recent_files_enabled()))
        })?;

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_recent_files_max_age_notify, move |s| {
            Ok(FfiDesktopSetting::RecentFilesMaxAgeDays(s.gtk_recent_files_max_age()))
        })?;

        Ok(())
    }
}

pub struct DesktopSettings {
    app_settings: Option<AppSettings>,
    gio_schema_and_settings: Option<(gio::SettingsSchema, gio::Settings)>,
    gio_settings_handlers: Vec<glib::SignalHandlerId>,
    display: gdk4::Display,
    display_settings_handlers: Vec<glib::SignalHandlerId>,
}

impl Drop for DesktopSettings {
    fn drop(&mut self) {
        if let Some((_, gio_settings)) = &mut self.gio_schema_and_settings {
            for handler_id in std::mem::take(&mut self.gio_settings_handlers) {
                gio_settings.disconnect(handler_id);
            }
        }
        for handler_id in std::mem::take(&mut self.display_settings_handlers) {
            self.display.disconnect(handler_id);
        }
    }
}

impl DesktopSettings {
    pub fn new(display: gdk4::Display) -> Self {
        let app_settings = gtk4::Settings::default().map(|gtk_settings| AppSettings {
            gtk_settings,
            settings_handlers: Vec::new(),
            initial_settings: Vec::new(),
        });
        let gio_schema_and_settings = gio::SettingsSchemaSource::default()
            .and_then(|source| source.lookup("org.gnome.desktop.interface", true))
            .map(|schema| (schema, gio::Settings::new("org.gnome.desktop.interface")));
        Self {
            app_settings,
            gio_schema_and_settings,
            gio_settings_handlers: Vec::new(),
            display,
            display_settings_handlers: Vec::new(),
        }
    }

    pub fn read_and_subscribe(&mut self, sender: impl Fn(FfiDesktopSetting) + Copy + 'static) -> anyhow::Result<Vec<FfiDesktopSetting>> {
        let mut initial_settings = if let Some(app_settings) = self.app_settings.as_mut() {
            app_settings.read_and_subscribe(sender)?;
            std::mem::take(&mut app_settings.initial_settings)
        } else {
            Vec::new()
        };

        if let Some((gio_schema, gio_settings)) = &self.gio_schema_and_settings {
            let initial_color_theme = if gio_schema.has_key("color-scheme") {
                let read = |s: &gio::Settings| XdgDesktopColorScheme::parse(&s.get("color-scheme"));
                self.gio_settings_handlers
                    .push(gio_settings.connect_changed(Some("color-scheme"), move |s, _name| {
                        sender(FfiDesktopSetting::ColorScheme(read(s).unwrap()));
                    }));
                read(gio_settings)?
            } else {
                XdgDesktopColorScheme::NoPreference
            };
            initial_settings.push(FfiDesktopSetting::ColorScheme(initial_color_theme));

            let initial_accent_color = if gio_schema.has_key("accent-color") {
                let read = |s: &gio::Settings| AccentColor::parse(&s.get("accent-color"));
                self.gio_settings_handlers
                    .push(gio_settings.connect_changed(Some("accent-color"), move |s, _name| {
                        sender(FfiDesktopSetting::AccentColor(read(s).unwrap()));
                    }));
                read(gio_settings)?
            } else {
                AccentColor::Unknown
            };
            initial_settings.push(FfiDesktopSetting::AccentColor(initial_accent_color));
        }

        let read_is_composited = |display: &gdk4::Display| FfiDesktopSetting::IsComposited(display.is_composited());
        self.display_settings_handlers
            .push(self.display.connect_composited_notify(move |display| {
                sender(read_is_composited(display));
            }));
        initial_settings.push(read_is_composited(&self.display));

        Ok(initial_settings)
    }

    pub fn set_prefer_dark_theme(&self, value: bool) {
        if let Some(app_settings) = self.app_settings.as_ref() {
            debug!("set_prefer_dark_theme: {value}");
            app_settings.gtk_settings.set_gtk_application_prefer_dark_theme(value);
        }
    }
}
