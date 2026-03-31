package org.jetbrains.desktop.gtk

import kotlin.time.Duration

public enum class ColorSchemeValue {
    /** No preference */
    NoPreference,

    /** Prefers dark appearance */
    PreferDark,

    /** Prefers light appearance */
    PreferLight,
}

public enum class AccentColorValue {
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

public enum class FontHintingValue {
    None,
    Slight,
    Medium,
    Full,
}

public enum class FontRgbaOrderValue {
    None,
    Rgb,
    Bgr,
    Vrgb,
    Vbgr,
}

public sealed class DesktopSetting {
    public data class AccentColor(val value: AccentColorValue) : DesktopSetting()

    public data class AudibleBell(val value: Boolean) : DesktopSetting()

    public data class ColorScheme(val value: ColorSchemeValue) : DesktopSetting()

    public data class CursorBlink(val value: Boolean) : DesktopSetting()

    /** Length of the cursor blink cycle */
    public data class CursorBlinkTime(val value: Duration) : DesktopSetting()

    /** Time after which the cursor stops blinking. */
    public data class CursorBlinkTimeout(val value: Duration) : DesktopSetting()

    public data class DoubleClickDistancePixels(val value: Int) : DesktopSetting()

    public data class DragAndDropDragThresholdPixels(val value: Int) : DesktopSetting()

    public data class DoubleClickInterval(val value: Duration) : DesktopSetting()

    public data class EnableAnimations(val value: Boolean) : DesktopSetting()

    public data class FontHinting(val value: FontHintingValue) : DesktopSetting()

    public data class FontRgbaOrder(val value: FontRgbaOrderValue) : DesktopSetting()

    public data class IsComposited(val value: Boolean) : DesktopSetting()

    public data class MiddleClickPaste(val value: Boolean) : DesktopSetting()

    public data class OverlayScrolling(val value: Boolean) : DesktopSetting()

    public data class PrimaryButtonWarpsSlider(val value: Boolean) : DesktopSetting()

    public data class RecentFilesEnabled(val value: Boolean) : DesktopSetting()
    public data class RecentFilesMaxAgeDays(val value: Int) : DesktopSetting()

    internal companion object;
}
