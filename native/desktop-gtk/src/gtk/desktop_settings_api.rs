use desktop_common::ffi_utils::BorrowedUtf8;

#[repr(C)]
#[derive(Debug, Default)]
pub enum XdgDesktopColorScheme {
    /// No preference
    #[default]
    NoPreference,
    /// Prefers dark appearance
    PreferDark,
    /// Prefers light appearance
    PreferLight,
}

#[repr(C)]
#[derive(Debug, Default)]
pub enum AccentColor {
    #[default]
    Unknown,
    Blue,
    Teal,
    Green,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
    Slate,
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontRgbaOrder {
    None,
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
#[derive(Debug)]
pub enum FfiDesktopSetting<'a> {
    AccentColor(AccentColor),
    AudibleBell(bool),
    ActionDoubleClickTitlebar(DesktopTitlebarAction),
    ActionRightClickTitlebar(DesktopTitlebarAction),
    ActionMiddleClickTitlebar(DesktopTitlebarAction),
    ColorScheme(XdgDesktopColorScheme),
    CursorBlink(bool),
    /// Length of the cursor blink cycle, in milliseconds.
    CursorBlinkTimeMs(i32),
    /// Time after which the cursor stops blinking.
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
    TitlebarLayout(BorrowedUtf8<'a>),
}
