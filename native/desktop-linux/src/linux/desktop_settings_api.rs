use desktop_common::ffi_utils::BorrowedArray;

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
pub enum DesktopTitlebarAction {
    Minimize,
    ToggleMaximize,
    Menu,
    None,
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
pub enum FfiDesktopSetting<'a> {
    AccentColor(Color),
    AudibleBell(bool),
    ActionDoubleClickTitlebar(DesktopTitlebarAction),
    ActionRightClickTitlebar(DesktopTitlebarAction),
    ActionMiddleClickTitlebar(DesktopTitlebarAction),
    ColorScheme(XdgDesktopColorScheme),
    CursorBlink(bool),

    /// Length of the cursor blink cycle, in milliseconds.
    CursorBlinkTimeMs(i32),

    /// Time after which the cursor stops blinking.
    CursorBlinkTimeoutMs(i32),

    CursorSize(i32),
    CursorTheme(BorrowedArray<'a, u8>),
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
    TitlebarLayout(BorrowedArray<'a, u8>),
}
