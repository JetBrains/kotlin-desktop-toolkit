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

public sealed class XdgDesktopSetting {
    public data class TitlebarLayout(val value: String) : XdgDesktopSetting()

    public data class DoubleClickInterval(val value: Duration) : XdgDesktopSetting()

    public data class ColorScheme(val value: ColorSchemeValue) : XdgDesktopSetting()

    public data class AccentColor(val value: Color) : XdgDesktopSetting()

    public data class FontAntialiasing(val value: FontAntialiasingValue) : XdgDesktopSetting()

    public data class FontHinting(val value: FontHintingValue) : XdgDesktopSetting()

    public data class FontRgbaOrder(val value: FontRgbaOrderValue) : XdgDesktopSetting()

    public data class CursorSize(val value: Int) : XdgDesktopSetting()

    public data class CursorTheme(val value: String) : XdgDesktopSetting()

    public data class CursorBlink(val value: Boolean) : XdgDesktopSetting()

    /** Length of the cursor blink cycle, in milliseconds. */
    public data class CursorBlinkTime(val value: Duration) : XdgDesktopSetting()

    /** Time after which the cursor stops blinking. */
    public data class CursorBlinkTimeout(val value: Duration) : XdgDesktopSetting()

    public data class OverlayScrolling(val value: Boolean) : XdgDesktopSetting()

    public data class AudibleBell(val value: Boolean) : XdgDesktopSetting()

    internal companion object;
}
