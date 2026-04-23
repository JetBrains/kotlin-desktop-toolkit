package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeActivationTokenResponse
import org.jetbrains.desktop.linux.generated.NativeAutoDropArray_u8
import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_FfiSupportedActionsForMime
import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_u32
import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.linux.generated.NativeBorrowedUtf8
import org.jetbrains.desktop.linux.generated.NativeColor
import org.jetbrains.desktop.linux.generated.NativeCommonFileDialogParams
import org.jetbrains.desktop.linux.generated.NativeDataTransferAvailableEvent
import org.jetbrains.desktop.linux.generated.NativeDataTransferCancelledEvent
import org.jetbrains.desktop.linux.generated.NativeDataTransferContent
import org.jetbrains.desktop.linux.generated.NativeDataTransferEvent
import org.jetbrains.desktop.linux.generated.NativeDragAndDropFinishedEvent
import org.jetbrains.desktop.linux.generated.NativeDragAndDropLeaveEvent
import org.jetbrains.desktop.linux.generated.NativeDragAndDropQueryData
import org.jetbrains.desktop.linux.generated.NativeDragIconDrawEvent
import org.jetbrains.desktop.linux.generated.NativeDropPerformedEvent
import org.jetbrains.desktop.linux.generated.NativeEvent
import org.jetbrains.desktop.linux.generated.NativeFfiDesktopSetting
import org.jetbrains.desktop.linux.generated.NativeFfiDragAndDropQueryResponse
import org.jetbrains.desktop.linux.generated.NativeFfiSupportedActionsForMime
import org.jetbrains.desktop.linux.generated.NativeFfiTransferDataResponse
import org.jetbrains.desktop.linux.generated.NativeFileChooserResponse
import org.jetbrains.desktop.linux.generated.NativeKeyDownEvent
import org.jetbrains.desktop.linux.generated.NativeKeyUpEvent
import org.jetbrains.desktop.linux.generated.NativeLogicalPoint
import org.jetbrains.desktop.linux.generated.NativeLogicalRect
import org.jetbrains.desktop.linux.generated.NativeLogicalSize
import org.jetbrains.desktop.linux.generated.NativeModifiersChangedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseDownEvent
import org.jetbrains.desktop.linux.generated.NativeMouseEnteredEvent
import org.jetbrains.desktop.linux.generated.NativeMouseExitedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseMovedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseUpEvent
import org.jetbrains.desktop.linux.generated.NativeNotificationClosedEvent
import org.jetbrains.desktop.linux.generated.NativeNotificationShownEvent
import org.jetbrains.desktop.linux.generated.NativeOpenFileDialogParams
import org.jetbrains.desktop.linux.generated.NativePhysicalSize
import org.jetbrains.desktop.linux.generated.NativeSaveFileDialogParams
import org.jetbrains.desktop.linux.generated.NativeScrollData
import org.jetbrains.desktop.linux.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.linux.generated.NativeSoftwareDrawData
import org.jetbrains.desktop.linux.generated.NativeTextInputAvailabilityEvent
import org.jetbrains.desktop.linux.generated.NativeTextInputContext
import org.jetbrains.desktop.linux.generated.NativeTextInputDeleteSurroundingTextData
import org.jetbrains.desktop.linux.generated.NativeTextInputEvent
import org.jetbrains.desktop.linux.generated.NativeTextInputPreeditStringData
import org.jetbrains.desktop.linux.generated.NativeWindowCapabilities
import org.jetbrains.desktop.linux.generated.NativeWindowCloseRequestEvent
import org.jetbrains.desktop.linux.generated.NativeWindowConfigureEvent
import org.jetbrains.desktop.linux.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.linux.generated.NativeWindowKeyboardEnterEvent
import org.jetbrains.desktop.linux.generated.NativeWindowKeyboardLeaveEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScreenChangeEvent
import org.jetbrains.desktop.linux.generated.desktop_linux_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout
import kotlin.experimental.or
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.DurationUnit
import kotlin.time.toDuration

/**
 * Converts UTF-8 offset to UTF-16 offset.
 */
public fun utf8OffsetToUtf16Offset(string: CharSequence, offset: Long): Int {
    if (offset == 0L) {
        return 0
    }
    var utf8Offset = offset
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
    width = NativeLogicalSize.width(s).toUInt(),
    height = NativeLogicalSize.height(s).toUInt(),
)

internal fun LogicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalSize.allocate(arena)
    NativeLogicalSize.width(result, width.toInt())
    NativeLogicalSize.height(result, height.toInt())
    return result
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment) = LogicalPoint(
    x = NativeLogicalPoint.x(s),
    y = NativeLogicalPoint.y(s),
)

internal fun LogicalPoint.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalPoint.allocate(arena)
    NativeLogicalPoint.x(result, x)
    NativeLogicalPoint.y(result, y)
    return result
}

internal fun LogicalRect.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalRect.allocate(arena)
    NativeLogicalRect.x(result, x.toInt())
    NativeLogicalRect.y(result, y.toInt())
    NativeLogicalRect.width(result, width.toInt())
    NativeLogicalRect.height(result, height.toInt())
    return result
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment) = PhysicalSize(
    width = NativePhysicalSize.width(s),
    height = NativePhysicalSize.height(s),
)

private fun keyModifiersFromNative(nativeModifiers: Int): Set<KeyModifiers> {
    val modifiers = buildSet {
        if (nativeModifiers and desktop_linux_h.NativeKeyModifiers_Ctrl() > 0) {
            add(KeyModifiers.Control)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifiers_Alt() > 0) {
            add(KeyModifiers.Alt)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifiers_Shift() > 0) {
            add(KeyModifiers.Shift)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifiers_CapsLock() > 0) {
            add(KeyModifiers.CapsLock)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifiers_Logo() > 0) {
            add(KeyModifiers.Logo)
        }
        if (nativeModifiers and desktop_linux_h.NativeKeyModifiers_NumLock() > 0) {
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

private fun WindowDecorationMode.Companion.fromNative(raw: Int): WindowDecorationMode {
    return when (raw) {
        desktop_linux_h.NativeWindowDecorationMode_Client() -> WindowDecorationMode.Client
        desktop_linux_h.NativeWindowDecorationMode_Server() -> WindowDecorationMode.Server
        else -> error("Unexpected Window decoration mode: $raw")
    }
}

internal fun WindowDecorationMode.toNative(): Int {
    return when (this) {
        WindowDecorationMode.Client -> desktop_linux_h.NativeWindowDecorationMode_Client()
        WindowDecorationMode.Server -> desktop_linux_h.NativeWindowDecorationMode_Server()
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

internal fun Set<TextInputContentHint>.toNative(): Int {
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
    NativeTextInputContext.hints(result, hints.toNative())
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

        val nativeArray = arena.allocateArray(ValueLayout.JAVA_BYTE, *this)
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
        NativeBorrowedUtf8.len(native, byteArray.size.toLong())

        val nativeArray = arena.allocateArray(ValueLayout.JAVA_BYTE, *byteArray)
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

internal fun Set<DragAndDropAction>.toNative(): Byte {
    var result = desktop_linux_h.NativeDragAndDropAction_None().toByte()
    for (e in this) {
        result = result or e.toNative()
    }
    return result
}

internal fun SupportedActionsForMime.toNative(result: MemorySegment, arena: Arena) {
    NativeFfiSupportedActionsForMime.supported_mime_type(result, supportedMimeType.toNativeUtf8(arena))
    NativeFfiSupportedActionsForMime.supported_actions(result, supportedActions.toNative())
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
        delta = NativeScrollData.delta(s),
        wheelValue120 = NativeScrollData.wheel_value120(s),
        isInverted = NativeScrollData.is_inverted(s),
        isStop = NativeScrollData.is_stop(s),
    )
}

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
            val nativeEvent = NativeEvent.desktop_setting_change(s)
            Event.DesktopSettingChange(
                setting = DesktopSetting.fromNative(nativeEvent),
            )
        }

        desktop_linux_h.NativeEvent_DataTransferAvailable() -> {
            val nativeEvent = NativeEvent.data_transfer_available(s)
            val mimeTypesString = readStringFromNativeU8Array(NativeDataTransferAvailableEvent.mime_types(nativeEvent))!!
            Event.DataTransferAvailable(
                dataSource = DataSource.fromNative(NativeDataTransferAvailableEvent.data_source(nativeEvent)),
                mimeTypes = mimeTypesString.split(","),
            )
        }
        desktop_linux_h.NativeEvent_DataTransferCancelled() -> {
            val nativeEvent = NativeEvent.data_transfer_cancelled(s)
            Event.DataTransferCancelled(
                dataSource = DataSource.fromNative(NativeDataTransferCancelledEvent.data_source(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_DataTransfer() -> {
            val nativeEvent = NativeEvent.data_transfer(s)
            val content = DataTransferContent.fromNative(NativeDataTransferEvent.content(nativeEvent))
            Event.DataTransfer(
                serial = NativeDataTransferEvent.serial(nativeEvent),
                content = content,
            )
        }
        desktop_linux_h.NativeEvent_DragAndDropLeave() -> {
            val nativeEvent = NativeEvent.drag_and_drop_leave(s)
            Event.DragAndDropLeave(windowId = NativeDragAndDropLeaveEvent.window_id(nativeEvent))
        }
        desktop_linux_h.NativeEvent_DropPerformed() -> {
            val nativeEvent = NativeEvent.drop_performed(s)
            Event.DropPerformed(
                windowId = NativeDropPerformedEvent.window_id(nativeEvent),
                content = DataTransferContent.fromNative(NativeDropPerformedEvent.content(nativeEvent)),
                action = DragAndDropAction.fromNative(NativeDropPerformedEvent.action(nativeEvent).toInt()),
                locationInWindow = LogicalPoint.fromNative(NativeDropPerformedEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_DragIconDraw() -> {
            val nativeEvent = NativeEvent.drag_icon_draw(s)
            Event.DragIconDraw(
                softwareDrawData = SoftwareDrawData.fromNative(NativeDragIconDrawEvent.software_draw_data(nativeEvent)),
                size = PhysicalSize.fromNative(NativeDragIconDrawEvent.physical_size(nativeEvent)),
                scale = NativeDragIconDrawEvent.scale(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_DragAndDropFinished() -> {
            val nativeEvent = NativeEvent.drag_and_drop_finished(s)
            Event.DragAndDropFinished(
                windowId = NativeDragAndDropFinishedEvent.window_id(nativeEvent),
                action = DragAndDropAction.fromNative(NativeDragAndDropFinishedEvent.action(nativeEvent).toInt()),
            )
        }
        desktop_linux_h.NativeEvent_FileChooserResponse() -> {
            val nativeEvent = NativeEvent.file_chooser_response(s)
            val filesString = readStringFromNativeU8Array(NativeFileChooserResponse.newline_separated_files(nativeEvent))
            Event.FileChooserResponse(
                requestId = RequestId.fromNativeField(NativeFileChooserResponse.request_id(nativeEvent)),
                files = filesString?.trimEnd()?.split("\r\n") ?: emptyList(),
            )
        }
        desktop_linux_h.NativeEvent_ActivationTokenResponse() -> {
            val nativeEvent = NativeEvent.activation_token_response(s)
            Event.ActivationTokenResponse(
                requestId = RequestId.fromNativeField(NativeActivationTokenResponse.request_id(nativeEvent)),
                token = readStringFromNativeU8Array(NativeActivationTokenResponse.token(nativeEvent))!!,
            )
        }
        desktop_linux_h.NativeEvent_KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            val nativeU8Array = NativeKeyDownEvent.characters(nativeEvent)
            val characters = readNativeU8Array(nativeU8Array)?.decodeToString()
            Event.KeyDown(
                keyCode = KeyCode(NativeKeyDownEvent.code(nativeEvent).toUInt()),
                characters = characters,
                key = KeySym(NativeKeyDownEvent.key(nativeEvent).toUInt()),
                isRepeat = NativeKeyDownEvent.is_repeat(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                key = KeySym(NativeKeyUpEvent.key(nativeEvent).toUInt()),
                keyCode = KeyCode(NativeKeyUpEvent.code(nativeEvent).toUInt()),
            )
        }
        desktop_linux_h.NativeEvent_TextInputAvailability() -> {
            val nativeEvent = NativeEvent.text_input(s)
            Event.TextInputAvailability(
                windowId = NativeTextInputAvailabilityEvent.window_id(nativeEvent),
                available = NativeTextInputAvailabilityEvent.available(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_TextInput() -> {
            val nativeEvent = NativeEvent.text_input(s)
            Event.TextInput(
                preeditStringData = if (NativeTextInputEvent.has_preedit_string(nativeEvent)) {
                    TextInputPreeditStringData.fromNative(NativeTextInputEvent.preedit_string(nativeEvent))
                } else {
                    null
                },
                commitStringData = if (NativeTextInputEvent.has_commit_string(nativeEvent)) {
                    TextInputCommitStringData(
                        text = readStringFromNativeU8Array(NativeTextInputEvent.commit_string(nativeEvent)),
                    )
                } else {
                    null
                },
                deleteSurroundingTextData = if (NativeTextInputEvent.has_delete_surrounding_text(nativeEvent)) {
                    TextInputDeleteSurroundingTextData.fromNative(NativeTextInputEvent.delete_surrounding_text(nativeEvent))
                } else {
                    null
                },
            )
        }
        desktop_linux_h.NativeEvent_ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            val nativeModifiers = NativeModifiersChangedEvent.modifiers(nativeEvent)
            Event.ModifiersChanged(modifiers = keyModifiersFromNative(nativeModifiers))
        }
        desktop_linux_h.NativeEvent_MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = NativeMouseMovedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseMovedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp.fromNative(NativeMouseMovedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseEntered() -> {
            val nativeEvent = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                windowId = NativeMouseEnteredEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseEnteredEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseExited() -> {
            val nativeEvent = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                windowId = NativeMouseExitedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseExitedEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                windowId = NativeMouseUpEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseUpEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseUpEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp.fromNative(NativeMouseUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                windowId = NativeMouseDownEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseDownEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDownEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp.fromNative(NativeMouseDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_NotificationClosed() -> {
            val nativeEvent = NativeEvent.notification_closed(s)
            Event.NotificationClosed(
                notificationId = NativeNotificationClosedEvent.notification_id(nativeEvent).toUInt(),
                action = readStringFromNativeU8Array(NativeNotificationClosedEvent.action(nativeEvent)),
                activationToken = readStringFromNativeU8Array(NativeNotificationClosedEvent.activation_token(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_NotificationShown() -> {
            val nativeEvent = NativeEvent.notification_shown(s)
            val nativeNotificationId = NativeNotificationShownEvent.notification_id(nativeEvent)
            Event.NotificationShown(
                requestId = RequestId.fromNativeField(NativeNotificationShownEvent.request_id(nativeEvent)),
                notificationId = if (nativeNotificationId == 0) null else nativeNotificationId.toUInt(),
            )
        }
        desktop_linux_h.NativeEvent_ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            val horizontalScroll = ScrollData.fromNative(NativeScrollWheelEvent.horizontal_scroll(nativeEvent))
            val verticalScroll = ScrollData.fromNative(NativeScrollWheelEvent.vertical_scroll(nativeEvent))
            Event.ScrollWheel(
                windowId = NativeScrollWheelEvent.window_id(nativeEvent),
                scrollingDeltaX = horizontalScroll.delta,
                scrollingDeltaY = verticalScroll.delta,
                locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp.fromNative(NativeScrollWheelEvent.timestamp(nativeEvent)),
                horizontalScroll = horizontalScroll,
                verticalScroll = verticalScroll,
            )
        } desktop_linux_h.NativeEvent_WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                windowId = NativeWindowScreenChangeEvent.window_id(nativeEvent),
                newScreenId = NativeWindowScreenChangeEvent.new_screen_id(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_WindowClosed() -> {
            val nativeEvent = NativeEvent.window_closed(s)
            Event.WindowClosed(
                windowId = NativeWindowConfigureEvent.window_id(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_WindowConfigure() -> {
            val nativeEvent = NativeEvent.window_configure(s)
            Event.WindowConfigure(
                windowId = NativeWindowConfigureEvent.window_id(nativeEvent),
                size = LogicalSize.fromNative(NativeWindowConfigureEvent.size(nativeEvent)),
                active = NativeWindowConfigureEvent.active(nativeEvent),
                maximized = NativeWindowConfigureEvent.maximized(nativeEvent),
                fullscreen = NativeWindowConfigureEvent.fullscreen(nativeEvent),
                tiledLeft = NativeWindowConfigureEvent.tiled_left(nativeEvent),
                tiledRight = NativeWindowConfigureEvent.tiled_right(nativeEvent),
                tiledTop = NativeWindowConfigureEvent.tiled_top(nativeEvent),
                tiledBottom = NativeWindowConfigureEvent.tiled_bottom(nativeEvent),
                decorationMode = WindowDecorationMode.fromNative(NativeWindowConfigureEvent.decoration_mode(nativeEvent)),
                capabilities = WindowCapabilities.fromNative(NativeWindowConfigureEvent.capabilities(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_WindowKeyboardEnter() -> {
            val nativeEvent = NativeEvent.window_keyboard_enter(s)

            val keyCodes = readNativeU32Array(NativeWindowKeyboardEnterEvent.raw(nativeEvent))!!.map { KeyCode(it.toUInt()) }
            val keySyms = readNativeU32Array(NativeWindowKeyboardEnterEvent.keysyms(nativeEvent))!!.map { KeySym(it.toUInt()) }

            Event.WindowKeyboardEnter(
                windowId = NativeWindowKeyboardEnterEvent.window_id(nativeEvent),
                keyCodes,
                keySyms,
            )
        }
        desktop_linux_h.NativeEvent_WindowKeyboardLeave() -> {
            val nativeEvent = NativeEvent.window_keyboard_leave(s)
            Event.WindowKeyboardLeave(windowId = NativeWindowKeyboardLeaveEvent.window_id(nativeEvent))
        }
        desktop_linux_h.NativeEvent_WindowCloseRequest() -> {
            val nativeEvent = NativeEvent.window_close_request(s)
            Event.WindowCloseRequest(windowId = NativeWindowCloseRequestEvent.window_id(nativeEvent))
        }
        desktop_linux_h.NativeEvent_WindowDraw() -> {
            val nativeEvent = NativeEvent.window_draw(s)
            Event.WindowDraw(
                windowId = NativeWindowDrawEvent.window_id(nativeEvent),
                softwareDrawData = SoftwareDrawData.fromNative(NativeWindowDrawEvent.software_draw_data(nativeEvent)),
                size = PhysicalSize.fromNative(NativeWindowDrawEvent.physical_size(nativeEvent)),
                scale = NativeWindowDrawEvent.scale(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_WindowScaleChanged() -> {
            val nativeEvent = NativeEvent.window_scale_changed(s)
            Event.WindowScaleChanged(
                windowId = NativeWindowScaleChangedEvent.window_id(nativeEvent),
                newScale = NativeWindowScaleChangedEvent.new_scale(nativeEvent),
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
