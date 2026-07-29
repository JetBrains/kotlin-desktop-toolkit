use crate::gtk::desktop_settings_api::{
    AccentColor, DesktopTitlebarAction, FfiDesktopSetting, FontHinting, FontRgbaOrder, XdgDesktopColorScheme,
};
use desktop_common::ffi_utils::BorrowedUtf8;
use gtk4::prelude::{DisplayExt, ObjectExt, SettingsExt, SettingsExtManual};
use gtk4::{gdk as gdk4, gio, glib};
use log::{debug, warn};

impl FontHinting {
    #[must_use]
    pub fn parse(value: &glib::GString) -> Option<Self> {
        match value.as_str() {
            "hintnone" => Some(Self::None),
            "hintslight" => Some(Self::Slight),
            "hintmedium" => Some(Self::Medium),
            "hintfull" => Some(Self::Full),
            _ => {
                warn!("Unknown font-hinting value {value}");
                None
            }
        }
    }
}

impl FontRgbaOrder {
    #[must_use]
    pub fn parse(value: &glib::GString) -> Option<Self> {
        match value.as_str() {
            "none" => Some(Self::None),
            "rgb" => Some(Self::Rgb),
            "bgr" => Some(Self::Bgr),
            "vrgb" => Some(Self::Vrgb),
            "vbgr" => Some(Self::Vbgr),
            _ => {
                warn!("Unknown font-rgba-order value {value}");
                None
            }
        }
    }
}

impl XdgDesktopColorScheme {
    #[must_use]
    fn parse(value: &String) -> Option<Self> {
        match value.as_str() {
            "default" => Some(Self::NoPreference),
            "prefer-dark" => Some(Self::PreferDark),
            "prefer-light" => Some(Self::PreferLight),
            _ => {
                warn!("Unknown XdgDesktopColorScheme value {value}");
                None
            }
        }
    }
}

impl AccentColor {
    #[must_use]
    fn parse(value: &String) -> Option<Self> {
        match value.as_str() {
            "blue" => Some(Self::Blue),
            "teal" => Some(Self::Teal),
            "green" => Some(Self::Green),
            "yellow" => Some(Self::Yellow),
            "orange" => Some(Self::Orange),
            "red" => Some(Self::Red),
            "pink" => Some(Self::Pink),
            "purple" => Some(Self::Purple),
            "slate" => Some(Self::Slate),
            _ => {
                warn!("Unknown accent color value {value}");
                None
            }
        }
    }
}

fn read_font_rgba_order(s: &gtk4::Settings) -> FontRgbaOrder {
    let value = s.gtk_xft_rgba();
    value.as_ref().and_then(FontRgbaOrder::parse).unwrap_or(FontRgbaOrder::None)
}

struct AppSettings {
    gtk_settings: gtk4::Settings,
    settings_handlers: Vec<glib::SignalHandlerId>,
    initial_settings: Vec<InternalDesktopSetting>,
}

impl Drop for AppSettings {
    fn drop(&mut self) {
        for handler_id in std::mem::take(&mut self.settings_handlers) {
            self.gtk_settings.disconnect(handler_id);
        }
    }
}

impl DesktopTitlebarAction {
    #[allow(clippy::needless_pass_by_value)]
    #[must_use]
    fn parse(value: glib::GString) -> Option<Self> {
        match value.as_str() {
            "toggle-maximize" | "toggle-maximize-horizontally" | "toggle-maximize-vertically" => Some(Self::ToggleMaximize),
            "minimize" | "lower" => Some(Self::Minimize),
            "none" => Some(Self::None),
            "menu" => Some(Self::Menu),
            _ => {
                warn!("Unknown DesktopTitlebarAction value {value}");
                None
            }
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
            InternalDesktopSetting::ColorScheme(v) => f(Self::ColorScheme(v)),
            InternalDesktopSetting::CursorBlink(v) => f(Self::CursorBlink(v)),
            InternalDesktopSetting::CursorBlinkTimeMs(v) => f(Self::CursorBlinkTimeMs(v)),
            InternalDesktopSetting::CursorBlinkTimeoutSeconds(v) => f(Self::CursorBlinkTimeoutSeconds(v)),
            InternalDesktopSetting::EnableAnimations(v) => f(Self::EnableAnimations(v)),
            InternalDesktopSetting::DoubleClickDistancePixels(v) => f(Self::DoubleClickDistancePixels(v)),
            InternalDesktopSetting::DoubleClickIntervalMs(v) => f(Self::DoubleClickIntervalMs(v)),
            InternalDesktopSetting::DragAndDropDragThresholdPixels(v) => f(Self::DragAndDropDragThresholdPixels(v)),
            InternalDesktopSetting::FontHinting(v) => f(Self::FontHinting(v)),
            InternalDesktopSetting::FontRgbaOrder(v) => f(Self::FontRgbaOrder(v)),
            InternalDesktopSetting::IsComposited(v) => f(Self::IsComposited(v)),
            InternalDesktopSetting::MiddleClickPaste(v) => f(Self::MiddleClickPaste(v)),
            InternalDesktopSetting::OverlayScrolling(v) => f(Self::OverlayScrolling(v)),
            InternalDesktopSetting::PrimaryButtonWarpsSlider(v) => f(Self::PrimaryButtonWarpsSlider(v)),
            InternalDesktopSetting::RecentFilesEnabled(v) => f(Self::RecentFilesEnabled(v)),
            InternalDesktopSetting::RecentFilesMaxAgeDays(v) => f(Self::RecentFilesMaxAgeDays(v)),
            InternalDesktopSetting::TitlebarLayout(v) => {
                f(FfiDesktopSetting::TitlebarLayout(BorrowedUtf8::new(&v)));
            }
        }
    }
}

#[derive(Debug)]
pub enum InternalDesktopSetting {
    AccentColor(AccentColor),
    AudibleBell(bool),
    ActionDoubleClickTitlebar(DesktopTitlebarAction),
    ActionRightClickTitlebar(DesktopTitlebarAction),
    ActionMiddleClickTitlebar(DesktopTitlebarAction),
    ColorScheme(XdgDesktopColorScheme),
    CursorBlink(bool),
    CursorBlinkTimeMs(i32),
    CursorBlinkTimeoutSeconds(i32),
    DoubleClickIntervalMs(i32),
    DoubleClickDistancePixels(i32),
    DragAndDropDragThresholdPixels(i32),
    EnableAnimations(bool),
    FontHinting(FontHinting),
    FontRgbaOrder(FontRgbaOrder),
    IsComposited(bool),
    MiddleClickPaste(bool),
    OverlayScrolling(bool),
    PrimaryButtonWarpsSlider(bool),
    RecentFilesEnabled(bool),
    RecentFilesMaxAgeDays(i32),
    TitlebarLayout(String),
}

impl AppSettings {
    #[allow(clippy::type_complexity)]
    fn read_and_subscribe_single(
        &mut self,
        sender: impl Fn(InternalDesktopSetting) + Copy + 'static,
        subscribe: fn(&gtk4::Settings, Box<dyn Fn(&gtk4::Settings)>) -> glib::SignalHandlerId,
        r: impl Fn(&gtk4::Settings) -> InternalDesktopSetting + 'static,
    ) {
        self.initial_settings.push(r(&self.gtk_settings));
        let handler_id = subscribe(
            &self.gtk_settings,
            Box::new(move |s| {
                let v = r(s);
                sender(v);
            }),
        );
        self.settings_handlers.push(handler_id);
    }

    #[allow(clippy::too_many_lines)]
    pub fn read_and_subscribe(&mut self, sender: impl Fn(InternalDesktopSetting) + Copy + 'static) {
        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_titlebar_double_click_notify, move |s| {
            InternalDesktopSetting::ActionDoubleClickTitlebar(
                s.gtk_titlebar_double_click()
                    .and_then(DesktopTitlebarAction::parse)
                    .unwrap_or(DesktopTitlebarAction::ToggleMaximize),
            )
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_titlebar_right_click_notify, move |s| {
            InternalDesktopSetting::ActionRightClickTitlebar(
                s.gtk_titlebar_right_click()
                    .and_then(DesktopTitlebarAction::parse)
                    .unwrap_or(DesktopTitlebarAction::Menu),
            )
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_titlebar_middle_click_notify, move |s| {
            InternalDesktopSetting::ActionMiddleClickTitlebar(
                s.gtk_titlebar_middle_click()
                    .and_then(DesktopTitlebarAction::parse)
                    .unwrap_or(DesktopTitlebarAction::None),
            )
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_double_click_time_notify, move |s| {
            InternalDesktopSetting::DoubleClickIntervalMs(s.gtk_double_click_time())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_xft_hintstyle_notify, move |s| {
            let value = s.gtk_xft_hintstyle();
            let hinting = value.as_ref().and_then(FontHinting::parse).unwrap_or(FontHinting::None);
            InternalDesktopSetting::FontHinting(hinting)
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_xft_rgba_notify, move |s| {
            InternalDesktopSetting::FontRgbaOrder(read_font_rgba_order(s))
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_cursor_blink_notify, move |s| {
            InternalDesktopSetting::CursorBlink(s.is_gtk_cursor_blink())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_cursor_blink_time_notify, move |s| {
            InternalDesktopSetting::CursorBlinkTimeMs(s.gtk_cursor_blink_time())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_cursor_blink_timeout_notify, move |s| {
            InternalDesktopSetting::CursorBlinkTimeoutSeconds(s.gtk_cursor_blink_timeout())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_overlay_scrolling_notify, move |s| {
            InternalDesktopSetting::OverlayScrolling(s.is_gtk_overlay_scrolling())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_enable_event_sounds_notify, move |s| {
            InternalDesktopSetting::AudibleBell(s.is_gtk_enable_event_sounds())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_enable_primary_paste_notify, move |s| {
            InternalDesktopSetting::MiddleClickPaste(s.is_gtk_enable_primary_paste())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_dnd_drag_threshold_notify, move |s| {
            InternalDesktopSetting::DragAndDropDragThresholdPixels(s.gtk_dnd_drag_threshold())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_double_click_distance_notify, move |s| {
            InternalDesktopSetting::DoubleClickDistancePixels(s.gtk_double_click_distance())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_enable_animations_notify, move |s| {
            InternalDesktopSetting::EnableAnimations(s.is_gtk_enable_animations())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_primary_button_warps_slider_notify, move |s| {
            InternalDesktopSetting::PrimaryButtonWarpsSlider(s.is_gtk_primary_button_warps_slider())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_recent_files_enabled_notify, move |s| {
            InternalDesktopSetting::RecentFilesEnabled(s.is_gtk_recent_files_enabled())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_recent_files_max_age_notify, move |s| {
            InternalDesktopSetting::RecentFilesMaxAgeDays(s.gtk_recent_files_max_age())
        });

        self.read_and_subscribe_single(sender, gtk4::Settings::connect_gtk_decoration_layout_notify, move |s| {
            InternalDesktopSetting::TitlebarLayout(
                s.gtk_decoration_layout()
                    .map_or_else(|| "menu:minimize,maximize,close".to_owned(), |v| v.as_str().to_owned()),
            )
        });
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

    pub fn read_and_subscribe(&mut self, sender: impl Fn(InternalDesktopSetting) + Copy + 'static) -> Vec<InternalDesktopSetting> {
        let mut initial_settings = if let Some(app_settings) = self.app_settings.as_mut() {
            app_settings.read_and_subscribe(sender);
            std::mem::take(&mut app_settings.initial_settings)
        } else {
            Vec::new()
        };

        if let Some((gio_schema, gio_settings)) = &self.gio_schema_and_settings {
            let initial_color_theme = if gio_schema.has_key("color-scheme") {
                let read = |s: &gio::Settings| XdgDesktopColorScheme::parse(&s.get("color-scheme"));
                self.gio_settings_handlers
                    .push(gio_settings.connect_changed(Some("color-scheme"), move |s, _name| {
                        sender(InternalDesktopSetting::ColorScheme(read(s).unwrap_or_default()));
                    }));
                read(gio_settings).unwrap_or_default()
            } else {
                XdgDesktopColorScheme::NoPreference
            };
            initial_settings.push(InternalDesktopSetting::ColorScheme(initial_color_theme));

            let initial_accent_color = if gio_schema.has_key("accent-color") {
                let read = |s: &gio::Settings| AccentColor::parse(&s.get("accent-color"));
                self.gio_settings_handlers
                    .push(gio_settings.connect_changed(Some("accent-color"), move |s, _name| {
                        sender(InternalDesktopSetting::AccentColor(read(s).unwrap_or_default()));
                    }));
                read(gio_settings).unwrap_or_default()
            } else {
                AccentColor::Unknown
            };
            initial_settings.push(InternalDesktopSetting::AccentColor(initial_accent_color));
        }

        let read_is_composited = |display: &gdk4::Display| InternalDesktopSetting::IsComposited(display.is_composited());
        self.display_settings_handlers
            .push(self.display.connect_composited_notify(move |display| {
                sender(read_is_composited(display));
            }));
        initial_settings.push(read_is_composited(&self.display));

        initial_settings
    }

    pub fn set_prefer_dark_theme(&self, value: bool) {
        if let Some(app_settings) = self.app_settings.as_ref() {
            debug!("set_prefer_dark_theme: {value}");
            app_settings.gtk_settings.set_gtk_application_prefer_dark_theme(value);
        }
    }
}
