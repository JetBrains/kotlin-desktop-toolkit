use desktop_common::ffi_utils::BorrowedUtf8;

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XdgDesktopColorScheme {
    /// No preference
    NoPreference,
    /// Prefers dark appearance
    PreferDark,
    /// Prefers light appearance
    PreferLight,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XdgDesktopContrast {
    /// No preference
    NoPreference,
    /// Higher contrast
    High,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum XdgDesktopReducedMotion {
    /// No preference
    NoPreference,
    /// Reduced motion
    ReducedMotion,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontAntialiasing {
    None,
    Grayscale,
    Rgba,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontHinting {
    None,
    Slight,
    Medium,
    Full,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontRgbaOrder {
    Rgb,
    Bgr,
    Vrgb,
    Vbgr,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesktopTitlebarAction {
    Minimize,
    ToggleMaximize,
    Menu,
    None,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

#[repr(C)]
#[derive(Debug)]
pub enum FfiDesktopSetting<'a> {
    AccentColor(Color),
    AudibleBell(bool),
    ActionDoubleClickTitlebar(DesktopTitlebarAction),
    ActionRightClickTitlebar(DesktopTitlebarAction),
    ActionMiddleClickTitlebar(DesktopTitlebarAction),
    ColorScheme(XdgDesktopColorScheme),
    Contrast(XdgDesktopContrast),
    CursorBlink(bool),

    /// Length of the cursor blink cycle, in milliseconds.
    CursorBlinkTimeMs(i32),

    /// Time after which the cursor stops blinking.
    CursorBlinkTimeoutMs(i32),

    CursorSize(i32),
    CursorTheme(BorrowedUtf8<'a>),
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
    ReducedMotion(XdgDesktopReducedMotion),
    TitlebarLayout(BorrowedUtf8<'a>),
}
