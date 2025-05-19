use desktop_common::ffi_utils::BorrowedStrPtr;

#[repr(C)]
#[derive(Debug)]
pub enum XdgDesktopColorScheme {
    /// No preference
    NoPreference,
    /// Prefers dark appearance
    PreferDark,
    /// Prefers light appearance
    PreferLight,
}

#[repr(C)]
#[derive(Debug)]
pub enum FontAntialiasing {
    None,
    Grayscale,
    Rgba,
}

#[repr(C)]
#[derive(Debug)]
pub enum FontHinting {
    None,
    Slight,
    Medium,
    Full,
}

#[repr(C)]
#[derive(Debug)]
pub enum FontRgbaOrder {
    Rgb,
    Bgr,
    Vrgb,
    Vbgr,
}

#[repr(C)]
#[derive(Debug)]
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub alpha: f64,
}

#[repr(C)]
#[derive(Debug)]
pub enum XdgDesktopSetting<'a> {
    TitlebarLayout(BorrowedStrPtr<'a>),
    DoubleClickIntervalMs(i32),
    ColorScheme(XdgDesktopColorScheme),
    AccentColor(Color),
    FontAntialiasing(FontAntialiasing),
    FontHinting(FontHinting),
    FontRgbaOrder(FontRgbaOrder),
    CursorBlink(bool),
    CursorSize(i32),
    CursorTheme(BorrowedStrPtr<'a>),

    /// Length of the cursor blink cycle, in milliseconds.
    CursorBlinkTimeMs(i32),

    /// Time after which the cursor stops blinking.
    CursorBlinkTimeoutMs(i32),

    OverlayScrolling(bool),

    AudibleBell(bool),
}
