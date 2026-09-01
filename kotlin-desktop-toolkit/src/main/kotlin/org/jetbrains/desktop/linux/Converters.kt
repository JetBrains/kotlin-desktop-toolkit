package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeAutoDropArray_u8
import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_BorrowedUtf8
import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_FfiSupportedActionsForMime
import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_u32
import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.linux.generated.NativeBorrowedUtf8
import org.jetbrains.desktop.linux.generated.NativeColor
import org.jetbrains.desktop.linux.generated.NativeCommonFileDialogParams
import org.jetbrains.desktop.linux.generated.NativeDataTransferContent
import org.jetbrains.desktop.linux.generated.NativeDragAndDropQueryData
import org.jetbrains.desktop.linux.generated.NativeDragIconDrawEvent
import org.jetbrains.desktop.linux.generated.NativeEvent
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeActivationTokenResponse_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeDataTransferAvailable_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeDataTransferCancelled_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeDataTransfer_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeDragAndDropFinished_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeDragAndDropLeave_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeDropPerformed_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeFileChooserResponse_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeKeyDown_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeKeyUp_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeModifiersChanged_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeMouseDown_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeMouseEntered_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeMouseExited_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeMouseMoved_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeMouseUp_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeNotificationClosed_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeNotificationShown_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeScrollWheel_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeTextInputAvailability_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeTextInput_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeWindowCloseRequest_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeWindowClosed_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeWindowKeyboardEnter_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeWindowKeyboardLeave_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeWindowScaleChanged_Body
import org.jetbrains.desktop.linux.generated.NativeEvent_NativeWindowScreenChange_Body
import org.jetbrains.desktop.linux.generated.NativeFfiDesktopSetting
import org.jetbrains.desktop.linux.generated.NativeFfiDragAndDropQueryResponse
import org.jetbrains.desktop.linux.generated.NativeFfiSupportedActionsForMime
import org.jetbrains.desktop.linux.generated.NativeFfiTransferDataResponse
import org.jetbrains.desktop.linux.generated.NativeLogicalPoint
import org.jetbrains.desktop.linux.generated.NativeLogicalRect
import org.jetbrains.desktop.linux.generated.NativeLogicalSize
import org.jetbrains.desktop.linux.generated.NativeOpenFileDialogParams
import org.jetbrains.desktop.linux.generated.NativePhysicalSize
import org.jetbrains.desktop.linux.generated.NativeSaveFileDialogParams
import org.jetbrains.desktop.linux.generated.NativeScrollData
import org.jetbrains.desktop.linux.generated.NativeSoftwareDrawData
import org.jetbrains.desktop.linux.generated.NativeTextInputContext
import org.jetbrains.desktop.linux.generated.NativeTextInputDeleteSurroundingTextData
import org.jetbrains.desktop.linux.generated.NativeTextInputPreeditStringData
import org.jetbrains.desktop.linux.generated.NativeWindowCapabilities
import org.jetbrains.desktop.linux.generated.NativeWindowConfigureData
import org.jetbrains.desktop.linux.generated.NativeWindowDecorationMode
import org.jetbrains.desktop.linux.generated.NativeWindowDecorationMode_NativeClient_Body
import org.jetbrains.desktop.linux.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.linux.generated.NativeWindowFrame
import org.jetbrains.desktop.linux.generated.NativeWindowFramePadding
import org.jetbrains.desktop.linux.generated.NativeWindowFrameResizerThickness
import org.jetbrains.desktop.linux.generated.NativeWindowFrameTiling
import org.jetbrains.desktop.linux.generated.desktop_linux_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout
import kotlin.streams.asSequence
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.DurationUnit
import kotlin.time.toDuration

/**
 * Converts UTF-8 offset to UTF-16 offset.
 */
public fun utf8OffsetToUtf16Offset(string: CharSequence, offset: UInt): Int {
    if (offset == 0U) {
        return 0
    }
    var utf8Offset = offset.toLong()
    var utf16Offset = 0
    for (codePoint in string.codePoints()) {
        utf8Offset -= when {
            codePoint < 128 -> 1
            codePoint < 2048 -> 2
            codePoint < 65536 -> 3
            else -> 4
        }

        utf16Offset += 1
        // Code points from the supplementary planes are encoded as a surrogate pair in utf-16,
        // meaning we'll have one extra utf-16 code unit for every code point in this range.
        if (codePoint >= 65536) utf16Offset += 1

        if (utf8Offset <= 0) break
    }

    return utf16Offset
}

internal fun splitCsv(s: String): List<String> {
    return if (s.isEmpty()) {
        emptyList()
    } else {
        s.split(",")
    }
}

internal fun LogicalSize.Companion.fromNative(s: MemorySegment) = LogicalSize(
    width = LogicalPixelsInt(NativeLogicalSize.width(s)),
    height = LogicalPixelsInt(NativeLogicalSize.height(s)),
)

internal fun LogicalSize?.toNative(arena: Arena): MemorySegment {
    val width = this?.width?.rawLogical ?: 0
    val height = this?.height?.rawLogical ?: 0
    val result = NativeLogicalSize.allocate(arena)
    NativeLogicalSize.width(result, width)
    NativeLogicalSize.height(result, height)
    return result
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment) = LogicalPoint(
    x = LogicalPixels(NativeLogicalPoint.x(s)),
    y = LogicalPixels(NativeLogicalPoint.y(s)),
)

internal fun LogicalPoint.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalPoint.allocate(arena)
    NativeLogicalPoint.x(result, x.rawLogical)
    NativeLogicalPoint.y(result, y.rawLogical)
    return result
}

internal fun LogicalRect.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalRect.allocate(arena)
    NativeLogicalRect.x(result, x.rawLogical)
    NativeLogicalRect.y(result, y.rawLogical)
    NativeLogicalRect.width(result, width.rawLogical)
    NativeLogicalRect.height(result, height.rawLogical)
    return result
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment) = PhysicalSize(
    width = PhysicalPixels(NativePhysicalSize.width(s)),
    height = PhysicalPixels(NativePhysicalSize.height(s)),
)

private fun bitmaskContains(mask: Int, value: Int): Boolean {
    return mask and value == value
}

private fun keyModifiersFromNative(nativeModifiers: Int): Set<KeyModifiers> {
    val modifiers = buildSet {
        if (bitmaskContains(nativeModifiers, desktop_linux_h.NativeKeyModifiers_Ctrl())) {
            add(KeyModifiers.Control)
        }
        if (bitmaskContains(nativeModifiers, desktop_linux_h.NativeKeyModifiers_Alt())) {
            add(KeyModifiers.Alt)
        }
        if (bitmaskContains(nativeModifiers, desktop_linux_h.NativeKeyModifiers_Shift())) {
            add(KeyModifiers.Shift)
        }
        if (bitmaskContains(nativeModifiers, desktop_linux_h.NativeKeyModifiers_CapsLock())) {
            add(KeyModifiers.CapsLock)
        }
        if (bitmaskContains(nativeModifiers, desktop_linux_h.NativeKeyModifiers_Logo())) {
            add(KeyModifiers.Logo)
        }
        if (bitmaskContains(nativeModifiers, desktop_linux_h.NativeKeyModifiers_NumLock())) {
            add(KeyModifiers.NumLock)
        }
    }

    return modifiers
}

internal fun PointerShape.toNative(): Int {
    return when (this) {
        PointerShape.Hidden -> desktop_linux_h.NativePointerShape_Hidden()
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

internal fun RenderingMode.toNative() = when (this) {
    RenderingMode.Auto -> desktop_linux_h.NativeRenderingMode_Auto()
    RenderingMode.Software -> desktop_linux_h.NativeRenderingMode_Software()
    RenderingMode.EGL -> desktop_linux_h.NativeRenderingMode_EGL()
}

internal fun WindowCapabilities.Companion.fromNative(s: MemorySegment) = WindowCapabilities(
    windowMenu = NativeWindowCapabilities.window_menu(s),
    maximize = NativeWindowCapabilities.maximize(s),
    fullscreen = NativeWindowCapabilities.fullscreen(s),
    minimize = NativeWindowCapabilities.minimize(s),
)

internal fun Color.Companion.fromNative(s: MemorySegment) = Color(
    red = NativeColor.red(s),
    green = NativeColor.green(s),
    blue = NativeColor.blue(s),
    alpha = NativeColor.alpha(s),
)

private fun DesktopTitlebarAction.Companion.fromNative(raw: Int): DesktopTitlebarAction {
    return when (raw) {
        desktop_linux_h.NativeDesktopTitlebarAction_Minimize() -> DesktopTitlebarAction.Minimize
        desktop_linux_h.NativeDesktopTitlebarAction_ToggleMaximize() -> DesktopTitlebarAction.ToggleMaximize
        desktop_linux_h.NativeDesktopTitlebarAction_Menu() -> DesktopTitlebarAction.Menu
        desktop_linux_h.NativeDesktopTitlebarAction_None() -> DesktopTitlebarAction.None
        else -> error("Unexpected desktop titlebar action value: $raw")
    }
}

private fun WindowDecorationMode.Companion.fromNative(native: MemorySegment): WindowDecorationMode {
    return when (val nativeTag = NativeWindowDecorationMode.tag(native)) {
        desktop_linux_h.NativeWindowDecorationMode_Client() -> WindowDecorationMode.Client.fromNative(
            NativeWindowDecorationMode.client(native),
        )
        desktop_linux_h.NativeWindowDecorationMode_Server() -> WindowDecorationMode.Server
        else -> error("Unexpected Window decoration mode: $nativeTag")
    }
}

internal fun DesktopSetting.Companion.fromNative(s: MemorySegment): DesktopSetting {
    return when (val nativeTag = NativeFfiDesktopSetting.tag(s)) {
        desktop_linux_h.NativeFfiDesktopSetting_TitlebarLayout() -> {
            DesktopSetting.TitlebarLayout(readStringFromNativeU8Array(NativeFfiDesktopSetting.titlebar_layout(s))!!)
        }

        desktop_linux_h.NativeFfiDesktopSetting_ActionDoubleClickTitlebar() -> {
            DesktopSetting.ActionDoubleClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeFfiDesktopSetting.action_double_click_titlebar(s)),
            )
        }

        desktop_linux_h.NativeFfiDesktopSetting_ActionRightClickTitlebar() -> {
            DesktopSetting.ActionRightClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeFfiDesktopSetting.action_right_click_titlebar(s)),
            )
        }

        desktop_linux_h.NativeFfiDesktopSetting_ActionMiddleClickTitlebar() -> {
            DesktopSetting.ActionMiddleClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeFfiDesktopSetting.action_middle_click_titlebar(s)),
            )
        }

        desktop_linux_h.NativeFfiDesktopSetting_DoubleClickIntervalMs() -> DesktopSetting.DoubleClickInterval(
            value = NativeFfiDesktopSetting.double_click_interval_ms(s).milliseconds,
        )

        desktop_linux_h.NativeFfiDesktopSetting_ColorScheme() -> DesktopSetting.ColorScheme(
            when (val raw = NativeFfiDesktopSetting.color_scheme(s)) {
                desktop_linux_h.NativeXdgDesktopColorScheme_NoPreference() -> ColorSchemeValue.NoPreference
                desktop_linux_h.NativeXdgDesktopColorScheme_PreferDark() -> ColorSchemeValue.PreferDark
                desktop_linux_h.NativeXdgDesktopColorScheme_PreferLight() -> ColorSchemeValue.PreferLight
                else -> {
                    Logger.warn { "Unexpected color scheme $raw" }
                    ColorSchemeValue.NoPreference
                }
            },
        )

        desktop_linux_h.NativeFfiDesktopSetting_Contrast() -> DesktopSetting.Contrast(
            when (val raw = NativeFfiDesktopSetting.contrast(s)) {
                desktop_linux_h.NativeXdgDesktopContrast_NoPreference() -> XdgDesktopContrast.NoPreference
                desktop_linux_h.NativeXdgDesktopContrast_High() -> XdgDesktopContrast.High
                else -> {
                    Logger.warn { "Unexpected contrast value $raw" }
                    XdgDesktopContrast.NoPreference
                }
            },
        )

        desktop_linux_h.NativeFfiDesktopSetting_ReducedMotion() -> DesktopSetting.ReducedMotion(
            when (val raw = NativeFfiDesktopSetting.reduced_motion(s)) {
                desktop_linux_h.NativeXdgDesktopReducedMotion_NoPreference() -> XdgDesktopReducedMotion.NoPreference
                desktop_linux_h.NativeXdgDesktopReducedMotion_ReducedMotion() -> XdgDesktopReducedMotion.ReducedMotion
                else -> {
                    Logger.warn { "Unexpected reduced motion value $raw" }
                    XdgDesktopReducedMotion.NoPreference
                }
            },
        )

        desktop_linux_h.NativeFfiDesktopSetting_AccentColor() -> DesktopSetting.AccentColor(
            Color.fromNative(NativeFfiDesktopSetting.accent_color(s)),
        )

        desktop_linux_h.NativeFfiDesktopSetting_FontAntialiasing() -> DesktopSetting.FontAntialiasing(
            when (val raw = NativeFfiDesktopSetting.font_antialiasing(s)) {
                desktop_linux_h.NativeFontAntialiasing_None() -> FontAntialiasingValue.None
                desktop_linux_h.NativeFontAntialiasing_Grayscale() -> FontAntialiasingValue.Grayscale
                desktop_linux_h.NativeFontAntialiasing_Rgba() -> FontAntialiasingValue.Rgba
                else -> error("Unexpected font aliasing $raw")
            },
        )

        desktop_linux_h.NativeFfiDesktopSetting_FontHinting() -> DesktopSetting.FontHinting(
            when (val raw = NativeFfiDesktopSetting.font_hinting(s)) {
                desktop_linux_h.NativeFontHinting_None() -> FontHintingValue.None
                desktop_linux_h.NativeFontHinting_Slight() -> FontHintingValue.Slight
                desktop_linux_h.NativeFontHinting_Medium() -> FontHintingValue.Medium
                desktop_linux_h.NativeFontHinting_Full() -> FontHintingValue.Full
                else -> error("Unexpected font hinting $raw")
            },
        )

        desktop_linux_h.NativeFfiDesktopSetting_FontRgbaOrder() -> DesktopSetting.FontRgbaOrder(
            when (val raw = NativeFfiDesktopSetting.font_rgba_order(s)) {
                desktop_linux_h.NativeFontRgbaOrder_Rgb() -> FontRgbaOrderValue.Rgb
                desktop_linux_h.NativeFontRgbaOrder_Bgr() -> FontRgbaOrderValue.Bgr
                desktop_linux_h.NativeFontRgbaOrder_Vrgb() -> FontRgbaOrderValue.Vrgb
                desktop_linux_h.NativeFontRgbaOrder_Vbgr() -> FontRgbaOrderValue.Vbgr
                else -> error("Unexpected font rgba order $raw")
            },
        )

        desktop_linux_h.NativeFfiDesktopSetting_CursorBlink() -> DesktopSetting.CursorBlink(NativeFfiDesktopSetting.cursor_blink(s))
        desktop_linux_h.NativeFfiDesktopSetting_CursorSize() -> DesktopSetting.CursorSize(
            NativeFfiDesktopSetting.cursor_size(s).toUInt(),
        )

        desktop_linux_h.NativeFfiDesktopSetting_CursorTheme() -> DesktopSetting.CursorTheme(
            readStringFromNativeU8Array(NativeFfiDesktopSetting.cursor_theme(s))!!,
        )

        desktop_linux_h.NativeFfiDesktopSetting_CursorBlinkTimeMs() -> DesktopSetting.CursorBlinkTime(
            NativeFfiDesktopSetting.cursor_blink_time_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )

        desktop_linux_h.NativeFfiDesktopSetting_CursorBlinkTimeoutMs() -> DesktopSetting.CursorBlinkTimeout(
            NativeFfiDesktopSetting.cursor_blink_timeout_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )

        desktop_linux_h.NativeFfiDesktopSetting_OverlayScrolling() -> DesktopSetting.OverlayScrolling(
            NativeFfiDesktopSetting.overlay_scrolling(s),
        )

        desktop_linux_h.NativeFfiDesktopSetting_AudibleBell() -> DesktopSetting.AudibleBell(NativeFfiDesktopSetting.audible_bell(s))
        desktop_linux_h.NativeFfiDesktopSetting_MiddleClickPaste() -> DesktopSetting.MiddleClickPaste(
            NativeFfiDesktopSetting.middle_click_paste(s),
        )

        desktop_linux_h.NativeFfiDesktopSetting_DragThresholdPixels() -> DesktopSetting.DragThresholdPixels(
            NativeFfiDesktopSetting.drag_threshold_pixels(s),
        )

        desktop_linux_h.NativeFfiDesktopSetting_EnableAnimations() -> DesktopSetting.EnableAnimations(
            NativeFfiDesktopSetting.enable_animations(s),
        )

        desktop_linux_h.NativeFfiDesktopSetting_RecentFilesEnabled() -> DesktopSetting.RecentFilesEnabled(
            NativeFfiDesktopSetting.recent_files_enabled(s),
        )

        desktop_linux_h.NativeFfiDesktopSetting_RecentFilesMaxAgeDays() -> DesktopSetting.RecentFilesMaxAgeDays(
            NativeFfiDesktopSetting.recent_files_max_age_days(s),
        )

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

internal fun WindowResizeEdge.toNative(): Int {
    return when (this) {
        WindowResizeEdge.Top -> desktop_linux_h.NativeWindowResizeEdge_Top()
        WindowResizeEdge.Bottom -> desktop_linux_h.NativeWindowResizeEdge_Bottom()
        WindowResizeEdge.Left -> desktop_linux_h.NativeWindowResizeEdge_Left()
        WindowResizeEdge.TopLeft -> desktop_linux_h.NativeWindowResizeEdge_TopLeft()
        WindowResizeEdge.BottomLeft -> desktop_linux_h.NativeWindowResizeEdge_BottomLeft()
        WindowResizeEdge.Right -> desktop_linux_h.NativeWindowResizeEdge_Right()
        WindowResizeEdge.TopRight -> desktop_linux_h.NativeWindowResizeEdge_TopRight()
        WindowResizeEdge.BottomRight -> desktop_linux_h.NativeWindowResizeEdge_BottomRight()
    }
}

internal fun TextInputContentPurpose.toNative(): Int {
    return when (this) {
        TextInputContentPurpose.Normal -> desktop_linux_h.NativeTextInputContentPurpose_Normal()
        TextInputContentPurpose.Alpha -> desktop_linux_h.NativeTextInputContentPurpose_Alpha()
        TextInputContentPurpose.Digits -> desktop_linux_h.NativeTextInputContentPurpose_Digits()
        TextInputContentPurpose.Number -> desktop_linux_h.NativeTextInputContentPurpose_Number()
        TextInputContentPurpose.Phone -> desktop_linux_h.NativeTextInputContentPurpose_Phone()
        TextInputContentPurpose.Url -> desktop_linux_h.NativeTextInputContentPurpose_Url()
        TextInputContentPurpose.Email -> desktop_linux_h.NativeTextInputContentPurpose_Email()
        TextInputContentPurpose.Name -> desktop_linux_h.NativeTextInputContentPurpose_Name()
        TextInputContentPurpose.Password -> desktop_linux_h.NativeTextInputContentPurpose_Password()
        TextInputContentPurpose.Pin -> desktop_linux_h.NativeTextInputContentPurpose_Pin()
        TextInputContentPurpose.Date -> desktop_linux_h.NativeTextInputContentPurpose_Date()
        TextInputContentPurpose.Time -> desktop_linux_h.NativeTextInputContentPurpose_Time()
        TextInputContentPurpose.Datetime -> desktop_linux_h.NativeTextInputContentPurpose_Datetime()
        TextInputContentPurpose.Terminal -> desktop_linux_h.NativeTextInputContentPurpose_Terminal()
    }
}

internal fun Set<TextInputContentHint>.toNativeTextInputContentHints(): Int {
    var nativeHints = 0
    for (hint in this) {
        nativeHints += when (hint) {
            TextInputContentHint.Completion -> desktop_linux_h.NativeTextInputContentHints_Completion()
            TextInputContentHint.Spellcheck -> desktop_linux_h.NativeTextInputContentHints_Spellcheck()
            TextInputContentHint.AutoCapitalization -> desktop_linux_h.NativeTextInputContentHints_AutoCapitalization()
            TextInputContentHint.Lowercase -> desktop_linux_h.NativeTextInputContentHints_Lowercase()
            TextInputContentHint.Uppercase -> desktop_linux_h.NativeTextInputContentHints_Uppercase()
            TextInputContentHint.Titlecase -> desktop_linux_h.NativeTextInputContentHints_Titlecase()
            TextInputContentHint.HiddenText -> desktop_linux_h.NativeTextInputContentHints_HiddenText()
            TextInputContentHint.SensitiveData -> desktop_linux_h.NativeTextInputContentHints_SensitiveData()
            TextInputContentHint.Latin -> desktop_linux_h.NativeTextInputContentHints_Latin()
            TextInputContentHint.Multiline -> desktop_linux_h.NativeTextInputContentHints_Multiline()
        }
    }
    return nativeHints
}

internal fun TextInputPreeditStringData.Companion.fromNative(s: MemorySegment): TextInputPreeditStringData {
    return TextInputPreeditStringData(
        text = readStringFromNativeU8Array(NativeTextInputPreeditStringData.text(s)),
        cursorBeginBytePos = NativeTextInputPreeditStringData.cursor_begin_byte_pos(s),
        cursorEndBytePos = NativeTextInputPreeditStringData.cursor_end_byte_pos(s),
    )
}

internal fun TextInputDeleteSurroundingTextData.Companion.fromNative(s: MemorySegment): TextInputDeleteSurroundingTextData {
    return TextInputDeleteSurroundingTextData(
        beforeLengthInBytes = NativeTextInputDeleteSurroundingTextData.before_length_in_bytes(s).toUInt(),
        afterLengthInBytes = NativeTextInputDeleteSurroundingTextData.after_length_in_bytes(s).toUInt(),
    )
}

internal fun TextInputContext.toNative(arena: Arena): MemorySegment {
    val result = NativeTextInputContext.allocate(arena)
    NativeTextInputContext.surrounding_text(result, surroundingText.toNativeUtf8(arena))
    NativeTextInputContext.cursor_codepoint_offset(result, cursorCodepointOffset.toShort())
    NativeTextInputContext.selection_start_codepoint_offset(result, selectionStartCodepointOffset.toShort())
    NativeTextInputContext.hints(result, hints.toNativeTextInputContentHints())
    NativeTextInputContext.content_purpose(result, contentPurpose.toNative())
    NativeTextInputContext.cursor_rectangle(result, cursorRectangle.toNative(arena))
    NativeTextInputContext.change_caused_by_input_method(result, changeCausedByInputMethod)
    return result
}

internal fun DataTransferContent.Companion.fromNative(s: MemorySegment): DataTransferContent? {
    val nativeU8Array = NativeDataTransferContent.data(s)
    val buf = readNativeU8Array(nativeU8Array) ?: return null
    val mimeType = readStringFromNativeU8Array(NativeDataTransferContent.mime_type(s))!!
    return DataTransferContent(mimeType = mimeType, data = buf)
}

internal fun DataSource.Companion.fromNative(nativeDataSource: Int): DataSource = when (nativeDataSource) {
    desktop_linux_h.NativeDataSource_Clipboard() -> DataSource.Clipboard
    desktop_linux_h.NativeDataSource_DragAndDrop() -> DataSource.DragAndDrop
    desktop_linux_h.NativeDataSource_PrimarySelection() -> DataSource.PrimarySelection
    else -> error("Unexpected data source type $nativeDataSource")
}

internal fun mimeTypesToNative(arena: Arena, mimeTypes: List<String>): MemorySegment {
    return mimeTypes.joinToString(",").toNativeUtf8(arena)
}

internal fun ByteArray?.toNative(arena: Arena): MemorySegment {
    val nativeDataArray = NativeBorrowedArray_u8.allocate(arena)
    if (this == null) {
        NativeBorrowedArray_u8.len(nativeDataArray, 0)
        NativeBorrowedArray_u8.ptr(nativeDataArray, MemorySegment.NULL)
    } else {
        NativeBorrowedArray_u8.len(nativeDataArray, size.toLong())

        val nativeArray = arena.allocateFrom(ValueLayout.JAVA_BYTE, *this)
        NativeBorrowedArray_u8.ptr(nativeDataArray, nativeArray)
    }

    return nativeDataArray
}

internal fun String?.toNativeUtf8(arena: Arena): MemorySegment {
    val native = NativeBorrowedUtf8.allocate(arena)
    if (this == null) {
        NativeBorrowedUtf8.len(native, 0)
        NativeBorrowedUtf8.ptr(native, MemorySegment.NULL)
    } else {
        val byteArray = encodeToByteArray()
        val byteArraySize = byteArray.size
        require(byteArraySize <= Application.MAX_STRING_SIZE_BYTES) {
            "String too long (max ${Application.MAX_STRING_SIZE_BYTES} bytes, but was $byteArraySize bytes): $this"
        }
        NativeBorrowedUtf8.len(native, byteArraySize.toLong())

        val nativeArray = arena.allocateFrom(ValueLayout.JAVA_BYTE, *byteArray)
        NativeBorrowedUtf8.ptr(native, nativeArray)
    }

    return native
}

internal fun ByteArray?.toNativeTransferDataResponse(arena: Arena, objId: Long): MemorySegment {
    val result = NativeFfiTransferDataResponse.allocate(arena)
    NativeFfiTransferDataResponse.obj_id(result, objId)
    NativeFfiTransferDataResponse.data(result, this.toNative(arena))
    return result
}

internal fun DragAndDropQueryData.Companion.fromNative(s: MemorySegment): DragAndDropQueryData {
    return DragAndDropQueryData(
        windowId = NativeDragAndDropQueryData.window_id(s),
        locationInWindow = LogicalPoint.fromNative(NativeDragAndDropQueryData.location_in_window(s)),
        mimeTypes = readNativeBorrowedUtf8Array(NativeDragAndDropQueryData.mime_types(s)),
        actions = dragAndDropActionsFromNative(NativeDragAndDropQueryData.actions(s)),
    )
}

internal fun DragAndDropAction?.toNative(): Byte = when (this) {
    null -> desktop_linux_h.NativeDragAndDropAction_None()
    DragAndDropAction.Copy -> desktop_linux_h.NativeDragAndDropAction_Copy()
    DragAndDropAction.Move -> desktop_linux_h.NativeDragAndDropAction_Move()
}.toByte()

internal fun DragAndDropAction.Companion.fromNative(nativeVal: Int): DragAndDropAction? = when (nativeVal) {
    desktop_linux_h.NativeDragAndDropAction_None() -> null
    desktop_linux_h.NativeDragAndDropAction_Copy() -> DragAndDropAction.Copy
    desktop_linux_h.NativeDragAndDropAction_Move() -> DragAndDropAction.Move
    else -> null
}

internal fun Set<DragAndDropAction>.toNativeDragAndDropActions(): Int {
    var result = desktop_linux_h.NativeDragAndDropAction_None()
    for (e in this) {
        result = result or e.toNative().toInt()
    }
    return result
}

internal fun dragAndDropActionsFromNative(raw: Int): Set<DragAndDropAction> {
    return buildSet {
        if (bitmaskContains(raw, desktop_linux_h.NativeDragAndDropAction_Copy())) {
            add(DragAndDropAction.Copy)
        }
        if (bitmaskContains(raw, desktop_linux_h.NativeDragAndDropAction_Move())) {
            add(DragAndDropAction.Move)
        }
    }
}

internal fun SupportedActionsForMime.toNative(result: MemorySegment, arena: Arena) {
    NativeFfiSupportedActionsForMime.supported_mime_type(result, supportedMimeType.toNativeUtf8(arena))
    NativeFfiSupportedActionsForMime.supported_actions(result, supportedActions.toNativeDragAndDropActions())
    NativeFfiSupportedActionsForMime.preferred_action(result, preferredAction.toNative())
}

internal fun DragAndDropQueryResponse.toNative(arena: Arena, objId: Long): MemorySegment {
    val nativeArray = NativeFfiSupportedActionsForMime.allocateArray(supportedActionsPerMime.size.toLong(), arena)
    supportedActionsPerMime.forEachIndexed { i, element ->
        element.toNative(NativeFfiSupportedActionsForMime.asSlice(nativeArray, i.toLong()), arena)
    }

    val nativeBorrowedArray = NativeBorrowedArray_FfiSupportedActionsForMime.allocate(arena)
    NativeBorrowedArray_FfiSupportedActionsForMime.len(nativeBorrowedArray, supportedActionsPerMime.size.toLong())
    NativeBorrowedArray_FfiSupportedActionsForMime.ptr(nativeBorrowedArray, nativeArray)

    val result = NativeFfiDragAndDropQueryResponse.allocate(arena)
    NativeFfiDragAndDropQueryResponse.supported_actions_per_mime(result, nativeBorrowedArray)
    NativeFfiDragAndDropQueryResponse.obj_id(result, objId)

    return result
}

internal fun ScrollData.Companion.fromNative(s: MemorySegment): ScrollData {
    return ScrollData(
        delta = LogicalPixels(NativeScrollData.delta(s)),
        wheelValue120 = NativeScrollData.wheel_value120(s),
        isInverted = NativeScrollData.is_inverted(s),
        isStop = NativeScrollData.is_stop(s),
    )
}

internal fun WindowDecorationMode.Client.Companion.fromNative(s: MemorySegment): WindowDecorationMode.Client {
    return WindowDecorationMode.Client(
        frame = WindowFrame.fromNative(NativeWindowDecorationMode_NativeClient_Body.frame(s)),
        tiled = WindowFrameTiling.fromNative(NativeWindowDecorationMode_NativeClient_Body.tiling(s)),
    )
}

internal fun WindowFrame.Companion.fromNative(s: MemorySegment): WindowFrame {
    return WindowFrame(
        padding = WindowFrame.Padding.fromNative(NativeWindowFrame.padding(s)),
        resizerThickness = WindowFrame.ResizerThickness.fromNative(NativeWindowFrame.resizer_thickness(s)),
    )
}

internal fun WindowFrame.toNative(arena: Arena): MemorySegment {
    val result = NativeWindowFrame.allocate(arena)
    NativeWindowFrame.padding(result, padding.toNative(arena))
    NativeWindowFrame.resizer_thickness(result, resizerThickness.toNative(arena))
    return result
}

internal fun WindowFrame.Padding.Companion.fromNative(s: MemorySegment) = WindowFrame.Padding(
    left = LogicalPixelsInt(NativeWindowFramePadding.left(s)),
    top = LogicalPixelsInt(NativeWindowFramePadding.top(s)),
    right = LogicalPixelsInt(NativeWindowFramePadding.right(s)),
    bottom = LogicalPixelsInt(NativeWindowFramePadding.bottom(s)),
)

internal fun WindowFrame.Padding.toNative(arena: Arena): MemorySegment {
    val result = NativeWindowFramePadding.allocate(arena)
    NativeWindowFramePadding.left(result, left.rawLogical)
    NativeWindowFramePadding.top(result, top.rawLogical)
    NativeWindowFramePadding.right(result, right.rawLogical)
    NativeWindowFramePadding.bottom(result, bottom.rawLogical)
    return result
}

internal fun WindowFrame.ResizerThickness.Companion.fromNative(s: MemorySegment) = WindowFrame.ResizerThickness(
    left = LogicalPixelsInt(NativeWindowFrameResizerThickness.left(s)),
    top = LogicalPixelsInt(NativeWindowFrameResizerThickness.top(s)),
    right = LogicalPixelsInt(NativeWindowFrameResizerThickness.right(s)),
    bottom = LogicalPixelsInt(NativeWindowFrameResizerThickness.bottom(s)),
)

internal fun WindowFrame.ResizerThickness.toNative(arena: Arena): MemorySegment {
    val result = NativeWindowFrameResizerThickness.allocate(arena)
    NativeWindowFrameResizerThickness.left(result, left.rawLogical)
    NativeWindowFrameResizerThickness.top(result, top.rawLogical)
    NativeWindowFrameResizerThickness.right(result, right.rawLogical)
    NativeWindowFrameResizerThickness.bottom(result, bottom.rawLogical)
    return result
}

internal fun WindowFrameTiling.Companion.fromNative(s: MemorySegment) = WindowFrameTiling(
    left = NativeWindowFrameTiling.left(s),
    top = NativeWindowFrameTiling.top(s),
    right = NativeWindowFrameTiling.right(s),
    bottom = NativeWindowFrameTiling.bottom(s),
)

internal fun readNativeAutoDropU8Array(nativeU8Array: MemorySegment): ByteArray? {
    val dataPtr = NativeAutoDropArray_u8.ptr(nativeU8Array)
    if (dataPtr == MemorySegment.NULL) {
        return null
    }
    val len = NativeAutoDropArray_u8.len(nativeU8Array)
    return dataPtr.asSlice(0, len).toArray(ValueLayout.JAVA_BYTE)
}

private fun readNativeU8Array(nativeU8Array: MemorySegment): ByteArray? {
    val dataPtr = NativeBorrowedArray_u8.ptr(nativeU8Array)
    if (dataPtr == MemorySegment.NULL) {
        return null
    }
    val len = NativeBorrowedArray_u8.len(nativeU8Array)
    return dataPtr.asSlice(0, len).toArray(ValueLayout.JAVA_BYTE)
}

internal fun readNativeBorrowedUtf8Array(nativeU8Array: MemorySegment): List<String> {
    val dataPtr = NativeBorrowedArray_BorrowedUtf8.ptr(nativeU8Array)
    if (dataPtr == MemorySegment.NULL) {
        return emptyList()
    }
    val len = NativeBorrowedArray_BorrowedUtf8.len(nativeU8Array)
    val slice = dataPtr.asSlice(0, len * NativeBorrowedUtf8.sizeof())
    val elements = slice.elements(NativeBorrowedUtf8.layout())
    val ret = elements.asSequence().mapNotNull(::readStringFromNativeU8Array).toList()
    return ret
}

internal fun readStringFromNativeU8Array(nativeU8Array: MemorySegment): String? {
    return readNativeU8Array(nativeU8Array)?.decodeToString()
}

private fun readNativeU32Array(nativeU32Array: MemorySegment): IntArray? {
    val dataPtr = NativeBorrowedArray_u32.ptr(nativeU32Array)
    if (dataPtr == MemorySegment.NULL) {
        return null
    }
    val len = NativeBorrowedArray_u32.len(nativeU32Array)
    return dataPtr.asSlice(0, len * 4).toArray(ValueLayout.JAVA_INT)
}

internal fun Event.Companion.fromNative(s: MemorySegment, app: Application): Event {
    return when (val raw = NativeEvent.tag(s)) {
        desktop_linux_h.NativeEvent_ApplicationStarted() -> {
            Event.ApplicationStarted
        }

        desktop_linux_h.NativeEvent_ApplicationWantsToTerminate() -> {
            Event.ApplicationWantsToTerminate
        }

        desktop_linux_h.NativeEvent_ApplicationWillTerminate() -> {
            Event.ApplicationWillTerminate
        }

        desktop_linux_h.NativeEvent_DisplayConfigurationChange() -> {
            Event.DisplayConfigurationChange(screens = app.allScreens())
        }

        desktop_linux_h.NativeEvent_DesktopSettingChange() -> {
            val body = NativeEvent.desktop_setting_change(s)
            Event.DesktopSettingChange(
                setting = DesktopSetting.fromNative(body),
            )
        }

        desktop_linux_h.NativeEvent_DataTransferAvailable() -> {
            val body = NativeEvent.data_transfer_available(s)
            val mimeTypesString = readStringFromNativeU8Array(NativeEvent_NativeDataTransferAvailable_Body.mime_types(body))!!
            Event.DataTransferAvailable(
                dataSource = DataSource.fromNative(NativeEvent_NativeDataTransferAvailable_Body.data_source(body)),
                mimeTypes = mimeTypesString.split(","),
            )
        }

        desktop_linux_h.NativeEvent_DataTransferCancelled() -> {
            val body = NativeEvent.data_transfer_cancelled(s)
            Event.DataTransferCancelled(
                dataSource = DataSource.fromNative(NativeEvent_NativeDataTransferCancelled_Body.data_source(body)),
            )
        }

        desktop_linux_h.NativeEvent_DataTransfer() -> {
            val body = NativeEvent.data_transfer(s)
            Event.DataTransfer(
                serial = NativeEvent_NativeDataTransfer_Body.serial(body),
                content = DataTransferContent.fromNative(NativeEvent_NativeDataTransfer_Body.content(body)),
            )
        }

        desktop_linux_h.NativeEvent_DragAndDropLeave() -> {
            val body = NativeEvent.drag_and_drop_leave(s)
            Event.DragAndDropLeave(windowId = NativeEvent_NativeDragAndDropLeave_Body.window_id(body))
        }

        desktop_linux_h.NativeEvent_DropPerformed() -> {
            val body = NativeEvent.drop_performed(s)
            Event.DropPerformed(
                windowId = NativeEvent_NativeDropPerformed_Body.window_id(body),
                content = DataTransferContent.fromNative(NativeEvent_NativeDropPerformed_Body.content(body)),
                action = DragAndDropAction.fromNative(NativeEvent_NativeDropPerformed_Body.action(body).toInt()),
                locationInWindow = LogicalPoint.fromNative(NativeEvent_NativeDropPerformed_Body.location_in_window(body)),
            )
        }

        desktop_linux_h.NativeEvent_DragIconDraw() -> {
            val body = NativeEvent.drag_icon_draw(s)
            Event.DragIconDraw(
                softwareDrawData = SoftwareDrawData.fromNative(NativeDragIconDrawEvent.software_draw_data(body)),
                size = PhysicalSize.fromNative(NativeDragIconDrawEvent.physical_size(body)),
                scale = Scale.fromValue120(NativeDragIconDrawEvent.scale(body)),
            )
        }

        desktop_linux_h.NativeEvent_DragAndDropFinished() -> {
            val body = NativeEvent.drag_and_drop_finished(s)
            Event.DragAndDropFinished(
                windowId = NativeEvent_NativeDragAndDropFinished_Body.window_id(body),
                action = DragAndDropAction.fromNative(NativeEvent_NativeDragAndDropFinished_Body.action(body).toInt()),
            )
        }

        desktop_linux_h.NativeEvent_FileChooserResponse() -> {
            val body = NativeEvent.file_chooser_response(s)
            val filesString = readStringFromNativeU8Array(NativeEvent_NativeFileChooserResponse_Body.newline_separated_files(body))
            Event.FileChooserResponse(
                requestId = RequestId.fromNativeField(NativeEvent_NativeFileChooserResponse_Body.request_id(body)),
                files = filesString?.trimEnd()?.split("\r\n") ?: emptyList(),
            )
        }

        desktop_linux_h.NativeEvent_ActivationTokenResponse() -> {
            val body = NativeEvent.activation_token_response(s)
            Event.ActivationTokenResponse(
                requestId = RequestId.fromNativeField(NativeEvent_NativeActivationTokenResponse_Body.request_id(body)),
                token = readStringFromNativeU8Array(NativeEvent_NativeActivationTokenResponse_Body.token(body))!!,
            )
        }

        desktop_linux_h.NativeEvent_KeyDown() -> {
            val body = NativeEvent.key_down(s)
            val nativeU8Array = NativeEvent_NativeKeyDown_Body.characters(body)
            val characters = readNativeU8Array(nativeU8Array)?.decodeToString()
            Event.KeyDown(
                serial = EventSerial.fromNative(NativeEvent_NativeKeyDown_Body.serial(body)),
                keyCode = KeyCode(NativeEvent_NativeKeyDown_Body.code(body).toUInt()),
                characters = characters,
                key = KeySym(NativeEvent_NativeKeyDown_Body.key(body).toUInt()),
                isRepeat = NativeEvent_NativeKeyDown_Body.is_repeat(body),
            )
        }

        desktop_linux_h.NativeEvent_KeyUp() -> {
            val body = NativeEvent.key_up(s)
            Event.KeyUp(
                serial = EventSerial.fromNative(NativeEvent_NativeKeyUp_Body.serial(body)),
                key = KeySym(NativeEvent_NativeKeyUp_Body.key(body).toUInt()),
                keyCode = KeyCode(NativeEvent_NativeKeyUp_Body.code(body).toUInt()),
            )
        }

        desktop_linux_h.NativeEvent_TextInputAvailability() -> {
            val body = NativeEvent.text_input(s)
            Event.TextInputAvailability(
                windowId = NativeEvent_NativeTextInputAvailability_Body.window_id(body),
                available = NativeEvent_NativeTextInputAvailability_Body.available(body),
            )
        }

        desktop_linux_h.NativeEvent_TextInput() -> {
            val body = NativeEvent.text_input(s)
            Event.TextInput(
                preeditStringData = if (NativeEvent_NativeTextInput_Body.has_preedit_string(body)) {
                    TextInputPreeditStringData.fromNative(NativeEvent_NativeTextInput_Body.preedit_string(body))
                } else {
                    null
                },
                commitStringData = if (NativeEvent_NativeTextInput_Body.has_commit_string(body)) {
                    TextInputCommitStringData(
                        text = readStringFromNativeU8Array(NativeEvent_NativeTextInput_Body.commit_string(body)),
                    )
                } else {
                    null
                },
                deleteSurroundingTextData = if (NativeEvent_NativeTextInput_Body.has_delete_surrounding_text(body)) {
                    TextInputDeleteSurroundingTextData.fromNative(NativeEvent_NativeTextInput_Body.delete_surrounding_text(body))
                } else {
                    null
                },
            )
        }

        desktop_linux_h.NativeEvent_ModifiersChanged() -> {
            val body = NativeEvent.modifiers_changed(s)
            Event.ModifiersChanged(
                serial = EventSerial.fromNative(NativeEvent_NativeModifiersChanged_Body.serial(body)),
                modifiers = keyModifiersFromNative(NativeEvent_NativeModifiersChanged_Body.modifiers(body)),
            )
        }

        desktop_linux_h.NativeEvent_MouseMoved() -> {
            val body = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = NativeEvent_NativeMouseMoved_Body.window_id(body),
                locationInWindow = LogicalPoint.fromNative(NativeEvent_NativeMouseMoved_Body.location_in_window(body)),
                timestamp = Timestamp.fromNative(NativeEvent_NativeMouseMoved_Body.timestamp(body)),
            )
        }

        desktop_linux_h.NativeEvent_MouseEntered() -> {
            val body = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                serial = EventSerial.fromNative(NativeEvent_NativeMouseEntered_Body.serial(body)),
                windowId = NativeEvent_NativeMouseEntered_Body.window_id(body),
                locationInWindow = LogicalPoint.fromNative(NativeEvent_NativeMouseEntered_Body.location_in_window(body)),
            )
        }

        desktop_linux_h.NativeEvent_MouseExited() -> {
            val body = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                serial = EventSerial.fromNative(NativeEvent_NativeMouseExited_Body.serial(body)),
                windowId = NativeEvent_NativeMouseExited_Body.window_id(body),
                locationInWindow = LogicalPoint.fromNative(NativeEvent_NativeMouseExited_Body.location_in_window(body)),
            )
        }

        desktop_linux_h.NativeEvent_MouseUp() -> {
            val body = NativeEvent.mouse_up(s)
            Event.MouseUp(
                serial = EventSerial.fromNative(NativeEvent_NativeMouseUp_Body.serial(body)),
                windowId = NativeEvent_NativeMouseUp_Body.window_id(body),
                button = MouseButton(NativeEvent_NativeMouseUp_Body.button(body)),
                locationInWindow = LogicalPoint.fromNative(NativeEvent_NativeMouseUp_Body.location_in_window(body)),
                timestamp = Timestamp.fromNative(NativeEvent_NativeMouseUp_Body.timestamp(body)),
            )
        }

        desktop_linux_h.NativeEvent_MouseDown() -> {
            val body = NativeEvent.mouse_down(s)
            Event.MouseDown(
                serial = EventSerial.fromNative(NativeEvent_NativeMouseDown_Body.serial(body)),
                windowId = NativeEvent_NativeMouseDown_Body.window_id(body),
                button = MouseButton(NativeEvent_NativeMouseDown_Body.button(body)),
                locationInWindow = LogicalPoint.fromNative(NativeEvent_NativeMouseDown_Body.location_in_window(body)),
                timestamp = Timestamp.fromNative(NativeEvent_NativeMouseDown_Body.timestamp(body)),
            )
        }

        desktop_linux_h.NativeEvent_NotificationClosed() -> {
            val body = NativeEvent.notification_closed(s)
            Event.NotificationClosed(
                notificationId = NativeEvent_NativeNotificationClosed_Body.notification_id(body).toUInt(),
                action = readStringFromNativeU8Array(NativeEvent_NativeNotificationClosed_Body.action(body)),
                activationToken = readStringFromNativeU8Array(NativeEvent_NativeNotificationClosed_Body.activation_token(body)),
            )
        }

        desktop_linux_h.NativeEvent_NotificationShown() -> {
            val body = NativeEvent.notification_shown(s)
            val nativeNotificationId = NativeEvent_NativeNotificationShown_Body.notification_id(body)
            Event.NotificationShown(
                requestId = RequestId.fromNativeField(NativeEvent_NativeNotificationShown_Body.request_id(body)),
                notificationId = if (nativeNotificationId == 0) null else nativeNotificationId.toUInt(),
            )
        }

        desktop_linux_h.NativeEvent_ScrollWheel() -> {
            val body = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                windowId = NativeEvent_NativeScrollWheel_Body.window_id(body),
                locationInWindow = LogicalPoint.fromNative(NativeEvent_NativeScrollWheel_Body.location_in_window(body)),
                timestamp = Timestamp.fromNative(NativeEvent_NativeScrollWheel_Body.timestamp(body)),
                horizontalScroll = ScrollData.fromNative(NativeEvent_NativeScrollWheel_Body.horizontal_scroll(body)),
                verticalScroll = ScrollData.fromNative(NativeEvent_NativeScrollWheel_Body.vertical_scroll(body)),
            )
        }

        desktop_linux_h.NativeEvent_WindowScreenChange() -> {
            val body = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                windowId = NativeEvent_NativeWindowScreenChange_Body.window_id(body),
                newScreenId = NativeEvent_NativeWindowScreenChange_Body.new_screen_id(body),
            )
        }

        desktop_linux_h.NativeEvent_WindowClosed() -> {
            val body = NativeEvent.window_closed(s)
            Event.WindowClosed(
                windowId = NativeEvent_NativeWindowClosed_Body.window_id(body),
            )
        }

        desktop_linux_h.NativeEvent_WindowConfigure() -> {
            val body = NativeEvent.window_configure(s)
            Event.WindowConfigure(
                windowId = NativeWindowConfigureData.window_id(body),
                size = LogicalSize.fromNative(NativeWindowConfigureData.size(body)),
                active = NativeWindowConfigureData.active(body),
                maximized = NativeWindowConfigureData.maximized(body),
                fullscreen = NativeWindowConfigureData.fullscreen(body),
                decorationMode = WindowDecorationMode.fromNative(NativeWindowConfigureData.decoration_mode(body)),
                capabilities = WindowCapabilities.fromNative(NativeWindowConfigureData.capabilities(body)),
            )
        }

        desktop_linux_h.NativeEvent_WindowKeyboardEnter() -> {
            val body = NativeEvent.window_keyboard_enter(s)

            val keyCodes = readNativeU32Array(NativeEvent_NativeWindowKeyboardEnter_Body.raw(body))!!.map { KeyCode(it.toUInt()) }
            val keySyms = readNativeU32Array(NativeEvent_NativeWindowKeyboardEnter_Body.keysyms(body))!!.map { KeySym(it.toUInt()) }

            Event.WindowKeyboardEnter(
                serial = EventSerial.fromNative(NativeEvent_NativeWindowKeyboardEnter_Body.serial(body)),
                windowId = NativeEvent_NativeWindowKeyboardEnter_Body.window_id(body),
                keyCodes = keyCodes,
                keySyms = keySyms,
            )
        }

        desktop_linux_h.NativeEvent_WindowKeyboardLeave() -> {
            val body = NativeEvent.window_keyboard_leave(s)
            Event.WindowKeyboardLeave(
                serial = EventSerial.fromNative(NativeEvent_NativeWindowKeyboardLeave_Body.serial(body)),
                windowId = NativeEvent_NativeWindowKeyboardLeave_Body.window_id(body),
            )
        }

        desktop_linux_h.NativeEvent_WindowCloseRequest() -> {
            val body = NativeEvent.window_close_request(s)
            Event.WindowCloseRequest(windowId = NativeEvent_NativeWindowCloseRequest_Body.window_id(body))
        }

        desktop_linux_h.NativeEvent_WindowDraw() -> {
            val body = NativeEvent.window_draw(s)
            Event.WindowDraw(
                windowId = NativeWindowDrawEvent.window_id(body),
                softwareDrawData = SoftwareDrawData.fromNative(NativeWindowDrawEvent.software_draw_data(body)),
                size = PhysicalSize.fromNative(NativeWindowDrawEvent.physical_size(body)),
            )
        }

        desktop_linux_h.NativeEvent_WindowScaleChanged() -> {
            val body = NativeEvent.window_scale_changed(s)
            Event.WindowScaleChanged(
                windowId = NativeEvent_NativeWindowScaleChanged_Body.window_id(body),
                newScale = Scale.fromValue120(NativeEvent_NativeWindowScaleChanged_Body.new_scale(body)),
            )
        }

        else -> {
            error("Unexpected Event tag $raw")
        }
    }
}

internal fun FileDialog.CommonDialogParams.toNative(arena: Arena): MemorySegment {
    val result = NativeCommonFileDialogParams.allocate(arena)
    NativeCommonFileDialogParams.modal(result, modal)
    NativeCommonFileDialogParams.title(result, title.toNativeUtf8(arena))
    NativeCommonFileDialogParams.accept_label(result, acceptLabel.toNativeUtf8(arena))
    NativeCommonFileDialogParams.current_folder(result, currentFolder.toNativeUtf8(arena))
    return result
}

internal fun FileDialog.OpenDialogParams.toNative(arena: Arena): MemorySegment {
    val result = NativeOpenFileDialogParams.allocate(arena)
    NativeOpenFileDialogParams.select_directories(result, selectDirectories)
    NativeOpenFileDialogParams.allows_multiple_selection(result, allowsMultipleSelections)
    return result
}

internal fun FileDialog.SaveDialogParams.toNative(arena: Arena): MemorySegment {
    val result = NativeSaveFileDialogParams.allocate(arena)
    NativeSaveFileDialogParams.name_field_string_value(
        result,
        nameFieldStringValue.toNativeUtf8(arena),
    )
    return result
}
