package org.jetbrains.desktop.linux

import kotlin.time.Duration

public enum class WindowResizeEdge {
    /** The top edge is being dragged. */
    Top,

    /** The bottom edge is being dragged. */
    Bottom,

    /* The left edge is being dragged. */
    Left,

    /** The top left corner is being dragged. */
    TopLeft,

    /** The bottom left corner is being dragged. */
    BottomLeft,

    /** The right edge is being dragged. */
    Right,

    /** The top right corner is being dragged. */
    TopRight,

    /** The bottom right corner is being dragged. */
    BottomRight,
}

public enum class ColorSchemeValue {
    /** No preference */
    NoPreference,

    /** Prefers dark appearance */
    PreferDark,

    /** Prefers light appearance */
    PreferLight,
}

public enum class FontAntialiasingValue {
    None,
    Grayscale,
    Rgba,
}

public enum class FontHintingValue {
    None,
    Slight,
    Medium,
    Full,
}

public enum class FontRgbaOrderValue {
    Rgb,
    Bgr,
    Vrgb,
    Vbgr,
}

public enum class DesktopTitlebarAction {
    Minimize,
    ToggleMaximize,
    Menu,
    None,
    ;

    internal companion object
}

public sealed class DesktopSetting {
    public data class TitlebarLayout(val value: String) : DesktopSetting()

    public data class ActionDoubleClickTitlebar(val value: DesktopTitlebarAction) : DesktopSetting()

    public data class ActionRightClickTitlebar(val value: DesktopTitlebarAction) : DesktopSetting()

    public data class ActionMiddleClickTitlebar(val value: DesktopTitlebarAction) : DesktopSetting()

    public data class DoubleClickInterval(val value: Duration) : DesktopSetting()

    public data class ColorScheme(val value: ColorSchemeValue) : DesktopSetting()

    public data class AccentColor(val value: Color) : DesktopSetting()

    public data class FontAntialiasing(val value: FontAntialiasingValue) : DesktopSetting()

    public data class FontHinting(val value: FontHintingValue) : DesktopSetting()

    public data class FontRgbaOrder(val value: FontRgbaOrderValue) : DesktopSetting()

    public data class CursorSize(val value: UInt) : DesktopSetting()

    public data class CursorTheme(val value: String) : DesktopSetting()

    public data class CursorBlink(val value: Boolean) : DesktopSetting()

    /** Length of the cursor blink cycle, in milliseconds. */
    public data class CursorBlinkTime(val value: Duration) : DesktopSetting()

    /** Time after which the cursor stops blinking. */
    public data class CursorBlinkTimeout(val value: Duration) : DesktopSetting()

    public data class OverlayScrolling(val value: Boolean) : DesktopSetting()

    public data class AudibleBell(val value: Boolean) : DesktopSetting()

    public data class MiddleClickPaste(val value: Boolean) : DesktopSetting()

    internal companion object;
}
