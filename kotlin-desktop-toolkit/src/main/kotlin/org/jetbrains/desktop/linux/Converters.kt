package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeColor
import org.jetbrains.desktop.linux.generated.NativeKeyModifiers
import org.jetbrains.desktop.linux.generated.NativeLogicalPoint
import org.jetbrains.desktop.linux.generated.NativeLogicalSize
import org.jetbrains.desktop.linux.generated.NativePhysicalSize
import org.jetbrains.desktop.linux.generated.NativeSoftwareDrawData
import org.jetbrains.desktop.linux.generated.NativeTitlebarButtonLayout
import org.jetbrains.desktop.linux.generated.NativeWindowCapabilities
import org.jetbrains.desktop.linux.generated.NativeXdgDesktopSetting
import org.jetbrains.desktop.linux.generated.desktop_linux_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import kotlin.time.DurationUnit
import kotlin.time.toDuration

internal fun LogicalSize.Companion.fromNative(s: MemorySegment) = LogicalSize(
    width = NativeLogicalSize.width(s).toFloat(),
    height = NativeLogicalSize.height(s).toFloat(),
)

internal fun LogicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalSize.allocate(arena)
    NativeLogicalSize.width(result, width.toDouble())
    NativeLogicalSize.height(result, height.toDouble())
    return result
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment) = LogicalPoint(
    x = NativeLogicalPoint.x(s).toFloat(),
    y = NativeLogicalPoint.y(s).toFloat(),
)

internal fun LogicalPoint.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalPoint.allocate(arena)
    NativeLogicalPoint.x(result, x.toDouble())
    NativeLogicalPoint.y(result, y.toDouble())
    return result
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment) = PhysicalSize(
    width = NativePhysicalSize.width(s),
    height = NativePhysicalSize.height(s),
)

internal fun KeyModifiers.Companion.fromNative(s: MemorySegment) = KeyModifiers(
    capsLock = NativeKeyModifiers.caps_lock(s),
    shift = NativeKeyModifiers.shift(s),
    control = NativeKeyModifiers.ctrl(s),
    alt = NativeKeyModifiers.alt(s),
    logo = NativeKeyModifiers.logo(s),
    numLock = NativeKeyModifiers.num_lock(s),
)

internal fun PointerShape.toNative(): Int {
    return when (this) {
        PointerShape.Default -> desktop_linux_h.NativePointerShape_Default()
        PointerShape.ContextMenu -> desktop_linux_h.NativePointerShape_ContextMenu()
        PointerShape.Help -> desktop_linux_h.NativePointerShape_Help()
        PointerShape.Pointer -> desktop_linux_h.NativePointerShape_Pointer()
        PointerShape.Progress -> desktop_linux_h.NativePointerShape_Progress()
        PointerShape.Wait -> desktop_linux_h.NativePointerShape_Wait()
        PointerShape.Cell -> desktop_linux_h.NativePointerShape_Cell()
        PointerShape.Crosshair -> desktop_linux_h.NativePointerShape_Crosshair()
        PointerShape.Text -> desktop_linux_h.NativePointerShape_Text()
        PointerShape.VerticalText -> desktop_linux_h.NativePointerShape_VerticalText()
        PointerShape.Alias -> desktop_linux_h.NativePointerShape_Alias()
        PointerShape.Copy -> desktop_linux_h.NativePointerShape_Copy()
        PointerShape.Move -> desktop_linux_h.NativePointerShape_Move()
        PointerShape.NoDrop -> desktop_linux_h.NativePointerShape_NoDrop()
        PointerShape.NotAllowed -> desktop_linux_h.NativePointerShape_NotAllowed()
        PointerShape.Grab -> desktop_linux_h.NativePointerShape_Grab()
        PointerShape.Grabbing -> desktop_linux_h.NativePointerShape_Grabbing()
        PointerShape.EResize -> desktop_linux_h.NativePointerShape_EResize()
        PointerShape.NResize -> desktop_linux_h.NativePointerShape_NResize()
        PointerShape.NeResize -> desktop_linux_h.NativePointerShape_NeResize()
        PointerShape.NwResize -> desktop_linux_h.NativePointerShape_NwResize()
        PointerShape.SResize -> desktop_linux_h.NativePointerShape_SResize()
        PointerShape.SeResize -> desktop_linux_h.NativePointerShape_SeResize()
        PointerShape.SwResize -> desktop_linux_h.NativePointerShape_SwResize()
        PointerShape.WResize -> desktop_linux_h.NativePointerShape_WResize()
        PointerShape.EwResize -> desktop_linux_h.NativePointerShape_EwResize()
        PointerShape.NsResize -> desktop_linux_h.NativePointerShape_NsResize()
        PointerShape.NeswResize -> desktop_linux_h.NativePointerShape_NeswResize()
        PointerShape.NwseResize -> desktop_linux_h.NativePointerShape_NwseResize()
        PointerShape.ColResize -> desktop_linux_h.NativePointerShape_ColResize()
        PointerShape.RowResize -> desktop_linux_h.NativePointerShape_RowResize()
        PointerShape.AllScroll -> desktop_linux_h.NativePointerShape_AllScroll()
        PointerShape.ZoomIn -> desktop_linux_h.NativePointerShape_ZoomIn()
        PointerShape.ZoomOut -> desktop_linux_h.NativePointerShape_ZoomOut()
    }
}

internal fun WindowCapabilities.Companion.fromNative(s: MemorySegment) = WindowCapabilities(
    windowMenu = NativeWindowCapabilities.window_menu(s),
    maximixe = NativeWindowCapabilities.maximixe(s),
    fullscreen = NativeWindowCapabilities.fullscreen(s),
    minimize = NativeWindowCapabilities.minimize(s),
)

internal fun Color.Companion.fromNative(s: MemorySegment) = Color(
    red = NativeColor.red(s),
    green = NativeColor.green(s),
    blue = NativeColor.blue(s),
    alpha = NativeColor.alpha(s),
)

internal fun XdgDesktopSetting.Companion.fromNative(s: MemorySegment): XdgDesktopSetting {
    val nativeTag = NativeXdgDesktopSetting.tag(s)
    return when (nativeTag) {
        desktop_linux_h.NativeXdgDesktopSetting_TitlebarLayout() -> {
            val nativeTitlebarLayout = NativeXdgDesktopSetting.titlebar_layout(s)
            XdgDesktopSetting.TitlebarLayout(
                layoutLeft = WindowButtonType.fromNativeArray(NativeTitlebarButtonLayout.left_side(nativeTitlebarLayout)),
                layoutRight = WindowButtonType.fromNativeArray(NativeTitlebarButtonLayout.right_side(nativeTitlebarLayout)),
            )
        }
        desktop_linux_h.NativeXdgDesktopSetting_DoubleClickIntervalMs() -> XdgDesktopSetting.DoubleClickInterval(
            intervalMs = NativeXdgDesktopSetting.double_click_interval_ms(s),
        )
        desktop_linux_h.NativeXdgDesktopSetting_ColorScheme() -> XdgDesktopSetting.ColorScheme(
            when (NativeXdgDesktopSetting.color_scheme(s)) {
                desktop_linux_h.NativeXdgDesktopColorScheme_NoPreference() -> ColorSchemeValue.NoPreference
                desktop_linux_h.NativeXdgDesktopColorScheme_PreferDark() -> ColorSchemeValue.PreferDark
                desktop_linux_h.NativeXdgDesktopColorScheme_PreferLight() -> ColorSchemeValue.PreferLight
                else -> error("Unexpected color scheme ${NativeXdgDesktopSetting.color_scheme(s)}")
            },
        )
        desktop_linux_h.NativeXdgDesktopSetting_AccentColor() -> XdgDesktopSetting.AccentColor(
            Color.fromNative(NativeXdgDesktopSetting.accent_color(s)),
        )
        desktop_linux_h.NativeXdgDesktopSetting_FontAntialiasing() -> XdgDesktopSetting.FontAntialiasing(
            when (NativeXdgDesktopSetting.font_antialiasing(s)) {
                desktop_linux_h.NativeFontAntialiasing_None() -> FontAntialiasingValue.None
                desktop_linux_h.NativeFontAntialiasing_Grayscale() -> FontAntialiasingValue.Grayscale
                desktop_linux_h.NativeFontAntialiasing_Rgba() -> FontAntialiasingValue.Rgba
                else -> error("Unexpected font aliasing ${NativeXdgDesktopSetting.font_antialiasing(s)}")
            },
        )
        desktop_linux_h.NativeXdgDesktopSetting_FontHinting() -> XdgDesktopSetting.FontHinting(
            when (NativeXdgDesktopSetting.font_hinting(s)) {
                desktop_linux_h.NativeFontHinting_None() -> FontHintingValue.None
                desktop_linux_h.NativeFontHinting_Slight() -> FontHintingValue.Slight
                desktop_linux_h.NativeFontHinting_Medium() -> FontHintingValue.Medium
                desktop_linux_h.NativeFontHinting_Full() -> FontHintingValue.Full
                else -> error("Unexpected font hinting ${NativeXdgDesktopSetting.font_hinting(s)}")
            },
        )
        desktop_linux_h.NativeXdgDesktopSetting_FontRgbaOrder() -> XdgDesktopSetting.FontRgbaOrder(
            when (NativeXdgDesktopSetting.font_rgba_order(s)) {
                desktop_linux_h.NativeFontRgbaOrder_Rgb() -> FontRgbaOrderValue.Rgb
                desktop_linux_h.NativeFontRgbaOrder_Bgr() -> FontRgbaOrderValue.Bgr
                desktop_linux_h.NativeFontRgbaOrder_Vrgb() -> FontRgbaOrderValue.Vrgb
                desktop_linux_h.NativeFontRgbaOrder_Vbgr() -> FontRgbaOrderValue.Vbgr
                else -> error("Unexpected font rgba order ${NativeXdgDesktopSetting.font_rgba_order(s)}")
            },
        )
        desktop_linux_h.NativeXdgDesktopSetting_CursorBlink() -> XdgDesktopSetting.CursorBlink(NativeXdgDesktopSetting.cursor_blink(s))
        desktop_linux_h.NativeXdgDesktopSetting_CursorBlinkTimeMs() -> XdgDesktopSetting.CursorBlinkTime(
            NativeXdgDesktopSetting.cursor_blink_time_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )
        desktop_linux_h.NativeXdgDesktopSetting_CursorBlinkTimeoutMs() -> XdgDesktopSetting.CursorBlinkTimeout(
            NativeXdgDesktopSetting.cursor_blink_timeout_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )
        desktop_linux_h.NativeXdgDesktopSetting_OverlayScrolling() -> XdgDesktopSetting.OverlayScrolling(
            NativeXdgDesktopSetting.overlay_scrolling(s),
        )
        desktop_linux_h.NativeXdgDesktopSetting_AudibleBell() -> XdgDesktopSetting.AudibleBell(NativeXdgDesktopSetting.audible_bell(s))
        else -> error("Unexpected setting $nativeTag")
    }
}

internal fun SoftwareDrawData.Companion.fromNative(nativeSoftwareDrawData: MemorySegment): SoftwareDrawData? {
    val nativeCanvas = NativeSoftwareDrawData.canvas(nativeSoftwareDrawData)
    if (nativeCanvas == MemorySegment.NULL) {
        return null
    }

    return SoftwareDrawData(canvas = nativeCanvas.address(), stride = NativeSoftwareDrawData.stride(nativeSoftwareDrawData))
}
