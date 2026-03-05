package org.jetbrains.desktop.gtk

import org.jetbrains.desktop.gtk.generated.NativeBorrowedArray_SupportedActionsForMime
import org.jetbrains.desktop.gtk.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.gtk.generated.NativeColor
import org.jetbrains.desktop.gtk.generated.NativeCommonFileDialogParams
import org.jetbrains.desktop.gtk.generated.NativeDataTransferAvailableEvent
import org.jetbrains.desktop.gtk.generated.NativeDataTransferCancelledEvent
import org.jetbrains.desktop.gtk.generated.NativeDataTransferContent
import org.jetbrains.desktop.gtk.generated.NativeDataTransferEvent
import org.jetbrains.desktop.gtk.generated.NativeDragAndDropFinishedEvent
import org.jetbrains.desktop.gtk.generated.NativeDragAndDropLeaveEvent
import org.jetbrains.desktop.gtk.generated.NativeDragAndDropQueryData
import org.jetbrains.desktop.gtk.generated.NativeDragAndDropQueryResponse
import org.jetbrains.desktop.gtk.generated.NativeDragIconDrawEvent
import org.jetbrains.desktop.gtk.generated.NativeDropPerformedEvent
import org.jetbrains.desktop.gtk.generated.NativeEvent
import org.jetbrains.desktop.gtk.generated.NativeFileChooserResponse
import org.jetbrains.desktop.gtk.generated.NativeKeyDownEvent
import org.jetbrains.desktop.gtk.generated.NativeKeyUpEvent
import org.jetbrains.desktop.gtk.generated.NativeLogicalPoint
import org.jetbrains.desktop.gtk.generated.NativeLogicalRect
import org.jetbrains.desktop.gtk.generated.NativeLogicalSize
import org.jetbrains.desktop.gtk.generated.NativeModifiersChangedEvent
import org.jetbrains.desktop.gtk.generated.NativeMouseDownEvent
import org.jetbrains.desktop.gtk.generated.NativeMouseEnteredEvent
import org.jetbrains.desktop.gtk.generated.NativeMouseExitedEvent
import org.jetbrains.desktop.gtk.generated.NativeMouseMovedEvent
import org.jetbrains.desktop.gtk.generated.NativeMouseUpEvent
import org.jetbrains.desktop.gtk.generated.NativeNotificationClosedEvent
import org.jetbrains.desktop.gtk.generated.NativeNotificationShownEvent
import org.jetbrains.desktop.gtk.generated.NativeOpenFileDialogParams
import org.jetbrains.desktop.gtk.generated.NativeOpenGlDrawData
import org.jetbrains.desktop.gtk.generated.NativePhysicalSize
import org.jetbrains.desktop.gtk.generated.NativeSaveFileDialogParams
import org.jetbrains.desktop.gtk.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.gtk.generated.NativeShouldRedraw
import org.jetbrains.desktop.gtk.generated.NativeSupportedActionsForMime
import org.jetbrains.desktop.gtk.generated.NativeTextInputContext
import org.jetbrains.desktop.gtk.generated.NativeTextInputDeleteSurroundingTextData
import org.jetbrains.desktop.gtk.generated.NativeTextInputEvent
import org.jetbrains.desktop.gtk.generated.NativeTextInputPreeditStringData
import org.jetbrains.desktop.gtk.generated.NativeWindowCapabilities
import org.jetbrains.desktop.gtk.generated.NativeWindowConfigureEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowKeyboardEnterEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowKeyboardLeaveEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowScreenChangeEvent
import org.jetbrains.desktop.gtk.generated.NativeXdgDesktopSetting
import org.jetbrains.desktop.gtk.generated.desktop_gtk_h
import org.jetbrains.desktop.linux.fromOptionalNativeString
import java.lang.foreign.Arena
import java.lang.foreign.MemoryLayout
import java.lang.foreign.MemorySegment
import kotlin.experimental.and
import kotlin.experimental.or
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.DurationUnit
import kotlin.time.toDuration

internal fun fromOptionalNativeString(s: MemorySegment): String? {
    return if (s == MemorySegment.NULL) null else s.getUtf8String(0)
}

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

public fun splitCsv(s: String): List<String> {
    return if (s.isEmpty()) {
        emptyList()
    } else {
        s.split(",")
    }
}

internal fun LogicalSize.Companion.fromNative(s: MemorySegment) = LogicalSize(
    width = NativeLogicalSize.width(s),
    height = NativeLogicalSize.height(s),
)

internal fun LogicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalSize.allocate(arena)
    NativeLogicalSize.width(result, width)
    NativeLogicalSize.height(result, height)
    return result
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment) = LogicalPoint(
    x = NativeLogicalPoint.x(s).toFloat(),
    y = NativeLogicalPoint.y(s).toFloat(),
)

internal fun LogicalRect.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalRect.allocate(arena)
    NativeLogicalRect.x(result, x)
    NativeLogicalRect.y(result, y)
    NativeLogicalRect.width(result, width)
    NativeLogicalRect.height(result, height)
    return result
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment) = PhysicalSize(
    width = NativePhysicalSize.width(s),
    height = NativePhysicalSize.height(s),
)

private fun keyModifiersFromNative(nativeModifiers: Byte): Set<KeyModifiers> {
    val modifiers = buildSet {
        if (nativeModifiers and desktop_gtk_h.NativeKeyModifier_Ctrl().toByte() > 0) {
            add(KeyModifiers.Control)
        }
        if (nativeModifiers and desktop_gtk_h.NativeKeyModifier_Alt().toByte() > 0) {
            add(KeyModifiers.Alt)
        }
        if (nativeModifiers and desktop_gtk_h.NativeKeyModifier_Shift().toByte() > 0) {
            add(KeyModifiers.Shift)
        }
        if (nativeModifiers and desktop_gtk_h.NativeKeyModifier_CapsLock().toByte() > 0) {
            add(KeyModifiers.CapsLock)
        }
        if (nativeModifiers and desktop_gtk_h.NativeKeyModifier_Logo().toByte() > 0) {
            add(KeyModifiers.Logo)
        }
        if (nativeModifiers and desktop_gtk_h.NativeKeyModifier_NumLock().toByte() > 0) {
            add(KeyModifiers.NumLock)
        }
    }

    return modifiers
}

internal fun PointerShape.toNative(): Int {
    return when (this) {
        PointerShape.Hidden -> desktop_gtk_h.NativePointerShape_Hidden()
        PointerShape.Default -> desktop_gtk_h.NativePointerShape_Default()
        PointerShape.ContextMenu -> desktop_gtk_h.NativePointerShape_ContextMenu()
        PointerShape.Help -> desktop_gtk_h.NativePointerShape_Help()
        PointerShape.Pointer -> desktop_gtk_h.NativePointerShape_Pointer()
        PointerShape.Progress -> desktop_gtk_h.NativePointerShape_Progress()
        PointerShape.Wait -> desktop_gtk_h.NativePointerShape_Wait()
        PointerShape.Cell -> desktop_gtk_h.NativePointerShape_Cell()
        PointerShape.Crosshair -> desktop_gtk_h.NativePointerShape_Crosshair()
        PointerShape.Text -> desktop_gtk_h.NativePointerShape_Text()
        PointerShape.VerticalText -> desktop_gtk_h.NativePointerShape_VerticalText()
        PointerShape.Alias -> desktop_gtk_h.NativePointerShape_Alias()
        PointerShape.Copy -> desktop_gtk_h.NativePointerShape_Copy()
        PointerShape.Move -> desktop_gtk_h.NativePointerShape_Move()
        PointerShape.NoDrop -> desktop_gtk_h.NativePointerShape_NoDrop()
        PointerShape.NotAllowed -> desktop_gtk_h.NativePointerShape_NotAllowed()
        PointerShape.Grab -> desktop_gtk_h.NativePointerShape_Grab()
        PointerShape.Grabbing -> desktop_gtk_h.NativePointerShape_Grabbing()
        PointerShape.EResize -> desktop_gtk_h.NativePointerShape_EResize()
        PointerShape.NResize -> desktop_gtk_h.NativePointerShape_NResize()
        PointerShape.NeResize -> desktop_gtk_h.NativePointerShape_NeResize()
        PointerShape.NwResize -> desktop_gtk_h.NativePointerShape_NwResize()
        PointerShape.SResize -> desktop_gtk_h.NativePointerShape_SResize()
        PointerShape.SeResize -> desktop_gtk_h.NativePointerShape_SeResize()
        PointerShape.SwResize -> desktop_gtk_h.NativePointerShape_SwResize()
        PointerShape.WResize -> desktop_gtk_h.NativePointerShape_WResize()
        PointerShape.EwResize -> desktop_gtk_h.NativePointerShape_EwResize()
        PointerShape.NsResize -> desktop_gtk_h.NativePointerShape_NsResize()
        PointerShape.NeswResize -> desktop_gtk_h.NativePointerShape_NeswResize()
        PointerShape.NwseResize -> desktop_gtk_h.NativePointerShape_NwseResize()
        PointerShape.ColResize -> desktop_gtk_h.NativePointerShape_ColResize()
        PointerShape.RowResize -> desktop_gtk_h.NativePointerShape_RowResize()
        PointerShape.AllScroll -> desktop_gtk_h.NativePointerShape_AllScroll()
        PointerShape.ZoomIn -> desktop_gtk_h.NativePointerShape_ZoomIn()
        PointerShape.ZoomOut -> desktop_gtk_h.NativePointerShape_ZoomOut()
    }
}

internal fun RenderingMode.toNative() = when (this) {
    RenderingMode.Auto -> desktop_gtk_h.NativeRenderingMode_Auto()
    RenderingMode.EGL -> desktop_gtk_h.NativeRenderingMode_EGL()
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
        desktop_gtk_h.NativeDesktopTitlebarAction_Minimize() -> DesktopTitlebarAction.Minimize
        desktop_gtk_h.NativeDesktopTitlebarAction_ToggleMaximize() -> DesktopTitlebarAction.ToggleMaximize
        desktop_gtk_h.NativeDesktopTitlebarAction_Menu() -> DesktopTitlebarAction.Menu
        desktop_gtk_h.NativeDesktopTitlebarAction_None() -> DesktopTitlebarAction.None
        else -> error("Unexpected desktop titlebar action value: $raw")
    }
}

private fun WindowDecorationMode.Companion.fromNative(raw: Int): WindowDecorationMode {
    return when (raw) {
        desktop_gtk_h.NativeWindowDecorationMode_Client() -> WindowDecorationMode.Client
        desktop_gtk_h.NativeWindowDecorationMode_Server() -> WindowDecorationMode.Server
        else -> error("Unexpected Window decoration mode: $raw")
    }
}

internal fun WindowDecorationMode.toNative(): Int {
    return when (this) {
        WindowDecorationMode.Client -> desktop_gtk_h.NativeWindowDecorationMode_Client()
        WindowDecorationMode.Server -> desktop_gtk_h.NativeWindowDecorationMode_Server()
    }
}

internal fun XdgDesktopSetting.Companion.fromNative(s: MemorySegment): XdgDesktopSetting {
    return when (val nativeTag = NativeXdgDesktopSetting.tag(s)) {
        desktop_gtk_h.NativeXdgDesktopSetting_TitlebarLayout() -> {
            XdgDesktopSetting.TitlebarLayout(NativeXdgDesktopSetting.titlebar_layout(s).getUtf8String(0))
        }

        desktop_gtk_h.NativeXdgDesktopSetting_ActionDoubleClickTitlebar() -> {
            XdgDesktopSetting.ActionDoubleClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeXdgDesktopSetting.action_double_click_titlebar(s)),
            )
        }

        desktop_gtk_h.NativeXdgDesktopSetting_ActionRightClickTitlebar() -> {
            XdgDesktopSetting.ActionRightClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeXdgDesktopSetting.action_right_click_titlebar(s)),
            )
        }

        desktop_gtk_h.NativeXdgDesktopSetting_ActionMiddleClickTitlebar() -> {
            XdgDesktopSetting.ActionMiddleClickTitlebar(
                DesktopTitlebarAction.fromNative(NativeXdgDesktopSetting.action_middle_click_titlebar(s)),
            )
        }

        desktop_gtk_h.NativeXdgDesktopSetting_DoubleClickIntervalMs() -> XdgDesktopSetting.DoubleClickInterval(
            value = NativeXdgDesktopSetting.double_click_interval_ms(s).milliseconds,
        )

        desktop_gtk_h.NativeXdgDesktopSetting_ColorScheme() -> XdgDesktopSetting.ColorScheme(
            when (NativeXdgDesktopSetting.color_scheme(s)) {
                desktop_gtk_h.NativeXdgDesktopColorScheme_NoPreference() -> ColorSchemeValue.NoPreference
                desktop_gtk_h.NativeXdgDesktopColorScheme_PreferDark() -> ColorSchemeValue.PreferDark
                desktop_gtk_h.NativeXdgDesktopColorScheme_PreferLight() -> ColorSchemeValue.PreferLight
                else -> error("Unexpected color scheme ${NativeXdgDesktopSetting.color_scheme(s)}")
            },
        )

        desktop_gtk_h.NativeXdgDesktopSetting_AccentColor() -> XdgDesktopSetting.AccentColor(
            Color.fromNative(NativeXdgDesktopSetting.accent_color(s)),
        )

        desktop_gtk_h.NativeXdgDesktopSetting_FontAntialiasing() -> XdgDesktopSetting.FontAntialiasing(
            when (NativeXdgDesktopSetting.font_antialiasing(s)) {
                desktop_gtk_h.NativeFontAntialiasing_None() -> FontAntialiasingValue.None
                desktop_gtk_h.NativeFontAntialiasing_Grayscale() -> FontAntialiasingValue.Grayscale
                desktop_gtk_h.NativeFontAntialiasing_Rgba() -> FontAntialiasingValue.Rgba
                else -> error("Unexpected font aliasing ${NativeXdgDesktopSetting.font_antialiasing(s)}")
            },
        )

        desktop_gtk_h.NativeXdgDesktopSetting_FontHinting() -> XdgDesktopSetting.FontHinting(
            when (NativeXdgDesktopSetting.font_hinting(s)) {
                desktop_gtk_h.NativeFontHinting_None() -> FontHintingValue.None
                desktop_gtk_h.NativeFontHinting_Slight() -> FontHintingValue.Slight
                desktop_gtk_h.NativeFontHinting_Medium() -> FontHintingValue.Medium
                desktop_gtk_h.NativeFontHinting_Full() -> FontHintingValue.Full
                else -> error("Unexpected font hinting ${NativeXdgDesktopSetting.font_hinting(s)}")
            },
        )

        desktop_gtk_h.NativeXdgDesktopSetting_FontRgbaOrder() -> XdgDesktopSetting.FontRgbaOrder(
            when (NativeXdgDesktopSetting.font_rgba_order(s)) {
                desktop_gtk_h.NativeFontRgbaOrder_Rgb() -> FontRgbaOrderValue.Rgb
                desktop_gtk_h.NativeFontRgbaOrder_Bgr() -> FontRgbaOrderValue.Bgr
                desktop_gtk_h.NativeFontRgbaOrder_Vrgb() -> FontRgbaOrderValue.Vrgb
                desktop_gtk_h.NativeFontRgbaOrder_Vbgr() -> FontRgbaOrderValue.Vbgr
                else -> error("Unexpected font rgba order ${NativeXdgDesktopSetting.font_rgba_order(s)}")
            },
        )

        desktop_gtk_h.NativeXdgDesktopSetting_CursorBlink() -> XdgDesktopSetting.CursorBlink(NativeXdgDesktopSetting.cursor_blink(s))
        desktop_gtk_h.NativeXdgDesktopSetting_CursorSize() -> XdgDesktopSetting.CursorSize(
            NativeXdgDesktopSetting.cursor_size(s).toUInt(),
        )
        desktop_gtk_h.NativeXdgDesktopSetting_CursorTheme() -> XdgDesktopSetting.CursorTheme(
            NativeXdgDesktopSetting.cursor_theme(s).getUtf8String(0),
        )

        desktop_gtk_h.NativeXdgDesktopSetting_CursorBlinkTimeMs() -> XdgDesktopSetting.CursorBlinkTime(
            NativeXdgDesktopSetting.cursor_blink_time_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )

        desktop_gtk_h.NativeXdgDesktopSetting_CursorBlinkTimeoutMs() -> XdgDesktopSetting.CursorBlinkTimeout(
            NativeXdgDesktopSetting.cursor_blink_timeout_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )

        desktop_gtk_h.NativeXdgDesktopSetting_OverlayScrolling() -> XdgDesktopSetting.OverlayScrolling(
            NativeXdgDesktopSetting.overlay_scrolling(s),
        )

        desktop_gtk_h.NativeXdgDesktopSetting_AudibleBell() -> XdgDesktopSetting.AudibleBell(NativeXdgDesktopSetting.audible_bell(s))
        desktop_gtk_h.NativeXdgDesktopSetting_MiddleClickPaste() -> XdgDesktopSetting.MiddleClickPaste(
            NativeXdgDesktopSetting.middle_click_paste(s),
        )

        else -> error("Unexpected setting $nativeTag")
    }
}

internal fun OpenGlDrawData.Companion.fromNative(nativeSoftwareDrawData: MemorySegment): OpenGlDrawData {
    val framebuffer = NativeOpenGlDrawData.framebuffer(nativeSoftwareDrawData)

    return OpenGlDrawData(framebuffer = framebuffer)
}

internal fun WindowResizeEdge.toNative(): Int {
    return when (this) {
        WindowResizeEdge.Top -> desktop_gtk_h.NativeWindowResizeEdge_Top()
        WindowResizeEdge.Bottom -> desktop_gtk_h.NativeWindowResizeEdge_Bottom()
        WindowResizeEdge.Left -> desktop_gtk_h.NativeWindowResizeEdge_Left()
        WindowResizeEdge.TopLeft -> desktop_gtk_h.NativeWindowResizeEdge_TopLeft()
        WindowResizeEdge.BottomLeft -> desktop_gtk_h.NativeWindowResizeEdge_BottomLeft()
        WindowResizeEdge.Right -> desktop_gtk_h.NativeWindowResizeEdge_Right()
        WindowResizeEdge.TopRight -> desktop_gtk_h.NativeWindowResizeEdge_TopRight()
        WindowResizeEdge.BottomRight -> desktop_gtk_h.NativeWindowResizeEdge_BottomRight()
    }
}

internal fun TextInputContentPurpose.toNative(): Int {
    return when (this) {
        TextInputContentPurpose.Normal -> desktop_gtk_h.NativeTextInputContentPurpose_Normal()
        TextInputContentPurpose.Alpha -> desktop_gtk_h.NativeTextInputContentPurpose_Alpha()
        TextInputContentPurpose.Digits -> desktop_gtk_h.NativeTextInputContentPurpose_Digits()
        TextInputContentPurpose.Number -> desktop_gtk_h.NativeTextInputContentPurpose_Number()
        TextInputContentPurpose.Phone -> desktop_gtk_h.NativeTextInputContentPurpose_Phone()
        TextInputContentPurpose.Url -> desktop_gtk_h.NativeTextInputContentPurpose_Url()
        TextInputContentPurpose.Email -> desktop_gtk_h.NativeTextInputContentPurpose_Email()
        TextInputContentPurpose.Name -> desktop_gtk_h.NativeTextInputContentPurpose_Name()
        TextInputContentPurpose.Password -> desktop_gtk_h.NativeTextInputContentPurpose_Password()
        TextInputContentPurpose.Pin -> desktop_gtk_h.NativeTextInputContentPurpose_Pin()
        TextInputContentPurpose.Terminal -> desktop_gtk_h.NativeTextInputContentPurpose_Terminal()
    }
}

internal fun Set<TextInputContextHint>.toNative(): Int {
    var nativeHints = 0

    for (hint in this) {
        nativeHints += when (hint) {
            TextInputContextHint.WordCompletion -> desktop_gtk_h.NativeTextInputContextHint_WordCompletion()
            TextInputContextHint.Spellcheck -> desktop_gtk_h.NativeTextInputContextHint_Spellcheck()
            TextInputContextHint.Lowercase -> desktop_gtk_h.NativeTextInputContextHint_Lowercase()
            TextInputContextHint.UppercaseChars -> desktop_gtk_h.NativeTextInputContextHint_UppercaseChars()
            TextInputContextHint.UppercaseWords -> desktop_gtk_h.NativeTextInputContextHint_UppercaseWords()
            TextInputContextHint.UppercaseSentences -> desktop_gtk_h.NativeTextInputContextHint_UppercaseSentences()
        }
    }
    return nativeHints
}

internal fun TextInputPreeditStringData.Companion.fromNative(s: MemorySegment): TextInputPreeditStringData {
    return TextInputPreeditStringData(
        text = fromOptionalNativeString(NativeTextInputPreeditStringData.text(s)),
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
    NativeTextInputContext.surrounding_text(result, arena.allocateUtf8String(surroundingText))
    NativeTextInputContext.cursor_codepoint_offset(result, cursorCodepointOffset.toShort())
    NativeTextInputContext.selection_start_codepoint_offset(result, selectionStartCodepointOffset.toShort())
    NativeTextInputContext.hints(result, hints.toNative())
    NativeTextInputContext.content_purpose(result, contentPurpose.toNative())
    NativeTextInputContext.cursor_rectangle(result, cursorRectangle.toNative(arena))
    return result
}

internal fun DataTransferContent.Companion.fromNative(s: MemorySegment): DataTransferContent? {
    val nativeU8Array = NativeDataTransferContent.data(s)
    val buf = readNativeU8Array(nativeU8Array) ?: return null
    val mimeType = NativeDataTransferContent.mime_type(s).getUtf8String(0)
    return DataTransferContent(mimeType = mimeType, data = buf)
}

internal fun DataSource.Companion.fromNative(nativeDataSource: Int): DataSource = when (nativeDataSource) {
    desktop_gtk_h.NativeDataSource_Clipboard() -> DataSource.Clipboard
    desktop_gtk_h.NativeDataSource_DragAndDrop() -> DataSource.DragAndDrop
    desktop_gtk_h.NativeDataSource_PrimarySelection() -> DataSource.PrimarySelection
    else -> error("Unexpected data source type $nativeDataSource")
}

internal fun mimeTypesToNative(arena: Arena, mimeTypes: List<String>): MemorySegment {
    return arena.allocateUtf8String(mimeTypes.joinToString(","))
}

internal fun ByteArray?.toNative(): MemorySegment {
    val arena = Arena.ofConfined()
    val nativeDataArray = NativeBorrowedArray_u8.allocate(arena)
    if (this == null) {
        NativeBorrowedArray_u8.len(nativeDataArray, 0)
        NativeBorrowedArray_u8.ptr(nativeDataArray, MemorySegment.NULL)
    } else {
        NativeBorrowedArray_u8.len(nativeDataArray, size.toLong())
        val nativeArray = arena.allocate(MemoryLayout.sequenceLayout(size.toLong(), desktop_gtk_h.C_CHAR))
        this.forEachIndexed { i, b ->
            nativeArray.setAtIndex(desktop_gtk_h.C_CHAR, i.toLong(), b)
        }

        NativeBorrowedArray_u8.ptr(nativeDataArray, nativeArray)

        NativeBorrowedArray_u8.deinit(
            nativeDataArray,
            NativeBorrowedArray_u8.deinit.allocate({ _, _ ->
                arena.close()
            }, arena),
        )
    }

    return nativeDataArray
}

internal fun DragAndDropQueryData.Companion.fromNative(s: MemorySegment): DragAndDropQueryData {
    return DragAndDropQueryData(
        windowId = NativeDragAndDropQueryData.window_id(s),
        locationInWindow = LogicalPoint.fromNative(NativeDragAndDropQueryData.location_in_window(s)),
    )
}

internal fun DragAndDropAction?.toNative(): Byte = when (this) {
    null -> desktop_gtk_h.NativeDragAndDropAction_None()
    DragAndDropAction.Copy -> desktop_gtk_h.NativeDragAndDropAction_Copy()
    DragAndDropAction.Move -> desktop_gtk_h.NativeDragAndDropAction_Move()
}.toByte()

internal fun DragAndDropAction.Companion.fromNative(nativeVal: Int): DragAndDropAction? = when (nativeVal) {
    desktop_gtk_h.NativeDragAndDropAction_None() -> null
    desktop_gtk_h.NativeDragAndDropAction_Copy() -> DragAndDropAction.Copy
    desktop_gtk_h.NativeDragAndDropAction_Move() -> DragAndDropAction.Move
    else -> null
}

internal fun Set<DragAndDropAction>.toNative(): Byte {
    var result = desktop_gtk_h.NativeDragAndDropAction_None().toByte()
    for (e in this) {
        result = result or e.toNative()
    }
    return result
}

private fun Int.toUTF32ByteArray(): ByteArray {
    return byteArrayOf(
        ((this shr 24) and 0xFF).toByte(),
        ((this shr 16) and 0xFF).toByte(),
        ((this shr 8) and 0xFF).toByte(),
        (this and 0xFF).toByte(),
    )
}

internal fun SupportedActionsForMime.toNative(result: MemorySegment, arena: Arena) {
    NativeSupportedActionsForMime.supported_mime_type(result, arena.allocateUtf8String(supportedMimeType))
    NativeSupportedActionsForMime.supported_actions(result, supportedActions.toNative())
    NativeSupportedActionsForMime.preferred_action(result, preferredAction.toNative())
}

internal fun DragAndDropQueryResponse.toNative(): MemorySegment {
    val arena = Arena.ofConfined()
    val result = NativeDragAndDropQueryResponse.allocate(arena)

    val nativeDataArray = NativeBorrowedArray_SupportedActionsForMime.allocate(arena)
    NativeBorrowedArray_SupportedActionsForMime.len(nativeDataArray, supportedActionsPerMime.size.toLong())
    val nativeArray = NativeSupportedActionsForMime.allocateArray(supportedActionsPerMime.size.toLong(), arena)
    supportedActionsPerMime.forEachIndexed { i, element ->
        element.toNative(NativeSupportedActionsForMime.asSlice(nativeArray, i.toLong()), arena)
    }

    NativeBorrowedArray_SupportedActionsForMime.ptr(nativeDataArray, nativeArray)

    NativeBorrowedArray_SupportedActionsForMime.deinit(
        nativeDataArray,
        NativeBorrowedArray_SupportedActionsForMime.deinit.allocate({ _, _ ->
            arena.close()
        }, arena),
    )

    NativeDragAndDropQueryResponse.supported_actions_per_mime(result, nativeDataArray)

    return result
}

private fun readNativeU8Array(nativeU8Array: MemorySegment): ByteArray? {
    val dataPtr = NativeBorrowedArray_u8.ptr(nativeU8Array)
    if (dataPtr == MemorySegment.NULL) {
        return null
    }
    val len = NativeBorrowedArray_u8.len(nativeU8Array)
    val buf = ByteArray(len.toInt())
    for (i in 0 until len) {
        buf[i.toInt()] = dataPtr.getAtIndex(desktop_gtk_h.C_CHAR, i)
    }
    return buf
}

// private fun readNativeU32Array(nativeU32Array: MemorySegment): List<UInt> {
//    val len = NativeBorrowedArray_u32.len(nativeU32Array)
//    val dataPtr = NativeBorrowedArray_u32.ptr(nativeU32Array)
//    val values = mutableListOf<UInt>()
//    for (i in 0 until len) {
//        val raw = dataPtr.getAtIndex(desktop_gtk_h.C_INT, i)
//        Logger.debug { "readNativeU32ArrayFor: len=$len : dataPtr=$dataPtr, value of index $i : $raw" }
//        values.add(raw.toUInt())
//    }
//    return values
// }

internal fun Event.Companion.fromNative(s: MemorySegment, app: Application): Event {
    return when (NativeEvent.tag(s)) {
        desktop_gtk_h.NativeEvent_ApplicationStarted() -> {
            Event.ApplicationStarted
        }
        desktop_gtk_h.NativeEvent_DisplayConfigurationChange() -> {
            Event.DisplayConfigurationChange(screens = app.allScreens())
        }
        desktop_gtk_h.NativeEvent_XdgDesktopSettingChange() -> {
            val nativeEvent = NativeEvent.xdg_desktop_setting_change(s)
            Event.XdgDesktopSettingChange(
                setting = XdgDesktopSetting.fromNative(nativeEvent),
            )
        }

        desktop_gtk_h.NativeEvent_DataTransferAvailable() -> {
            val nativeEvent = NativeEvent.data_transfer_available(s)
            val mimeTypesString = NativeDataTransferAvailableEvent.mime_types(nativeEvent).getUtf8String(0)
            Event.DataTransferAvailable(
                dataSource = DataSource.fromNative(NativeDataTransferAvailableEvent.data_source(nativeEvent)),
                mimeTypes = splitCsv(mimeTypesString),
            )
        }
        desktop_gtk_h.NativeEvent_DataTransferCancelled() -> {
            val nativeEvent = NativeEvent.data_transfer_cancelled(s)
            Event.DataTransferCancelled(
                dataSource = DataSource.fromNative(NativeDataTransferCancelledEvent.data_source(nativeEvent)),
            )
        }
        desktop_gtk_h.NativeEvent_DataTransfer() -> {
            val nativeEvent = NativeEvent.data_transfer(s)
            val content = DataTransferContent.fromNative(NativeDataTransferEvent.content(nativeEvent))
            Event.DataTransfer(
                serial = NativeDataTransferEvent.serial(nativeEvent),
                content = content,
            )
        }
        desktop_gtk_h.NativeEvent_DragAndDropLeave() -> {
            val nativeEvent = NativeEvent.drag_and_drop_leave(s)
            Event.DragAndDropLeave(windowId = NativeDragAndDropLeaveEvent.window_id(nativeEvent))
        }
        desktop_gtk_h.NativeEvent_DropPerformed() -> {
            val nativeEvent = NativeEvent.drop_performed(s)
            Event.DropPerformed(
                windowId = NativeDropPerformedEvent.window_id(nativeEvent),
                content = DataTransferContent.fromNative(NativeDropPerformedEvent.content(nativeEvent)),
                action = DragAndDropAction.fromNative(NativeDropPerformedEvent.action(nativeEvent).toInt()),
            )
        }
        desktop_gtk_h.NativeEvent_DragIconDraw() -> {
            val nativeEvent = NativeEvent.drag_icon_draw(s)
            Event.DragIconDraw(
                openGlDrawData = OpenGlDrawData.fromNative(NativeDragIconDrawEvent.opengl_draw_data(nativeEvent)),
                size = PhysicalSize.fromNative(NativeDragIconDrawEvent.physical_size(nativeEvent)),
                scale = NativeDragIconDrawEvent.scale(nativeEvent),
            )
        }
        desktop_gtk_h.NativeEvent_DragAndDropFinished() -> {
            val nativeEvent = NativeEvent.drag_and_drop_finished(s)
            Event.DragAndDropFinished(
                windowId = NativeDragAndDropFinishedEvent.window_id(nativeEvent),
                action = DragAndDropAction.fromNative(NativeDragAndDropFinishedEvent.action(nativeEvent).toInt()),
            )
        }
        desktop_gtk_h.NativeEvent_FileChooserResponse() -> {
            val nativeEvent = NativeEvent.file_chooser_response(s)
            val filesString = fromOptionalNativeString(NativeFileChooserResponse.newline_separated_files(nativeEvent))
            Event.FileChooserResponse(
                requestId = RequestId.fromNativeField(NativeFileChooserResponse.request_id(nativeEvent)),
                files = filesString?.trimEnd()?.split("\r\n") ?: emptyList(),
            )
        }
        desktop_gtk_h.NativeEvent_KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            val hasCharacter = NativeKeyDownEvent.has_character(nativeEvent)
            val rawCharacter = NativeKeyDownEvent.character(nativeEvent)

            Event.KeyDown(
                windowId = NativeKeyDownEvent.window_id(nativeEvent),
                keyCode = KeyCode(NativeKeyDownEvent.code(nativeEvent).toUInt()),
                characters = if (hasCharacter) String(rawCharacter.toUTF32ByteArray(), Charsets.UTF_32) else null,
                key = KeySym(NativeKeyDownEvent.key(nativeEvent).toUInt()),
                modifiers = keyModifiersFromNative(NativeKeyDownEvent.modifiers(nativeEvent)),
                isRepeat = NativeKeyDownEvent.is_repeat(nativeEvent),
            )
        }
        desktop_gtk_h.NativeEvent_KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                windowId = NativeKeyUpEvent.window_id(nativeEvent),
                key = KeySym(NativeKeyUpEvent.key(nativeEvent).toUInt()),
                keyCode = KeyCode(NativeKeyUpEvent.code(nativeEvent).toUInt()),
            )
        }
        desktop_gtk_h.NativeEvent_TextInput() -> {
            val nativeEvent = NativeEvent.text_input(s)
            Event.TextInput(
                windowId = NativeTextInputEvent.window_id(nativeEvent),
                preeditStringData = if (NativeTextInputEvent.has_preedit_string(nativeEvent)) {
                    TextInputPreeditStringData.fromNative(NativeTextInputEvent.preedit_string(nativeEvent))
                } else {
                    null
                },
                commitStringData = if (NativeTextInputEvent.has_commit_string(nativeEvent)) {
                    TextInputCommitStringData(
                        text = fromOptionalNativeString(NativeTextInputEvent.commit_string(nativeEvent)),
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
        desktop_gtk_h.NativeEvent_ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            val nativeModifiers = NativeModifiersChangedEvent.modifiers(nativeEvent)
            Event.ModifiersChanged(
                windowId = NativeModifiersChangedEvent.window_id(nativeEvent),
                modifiers = keyModifiersFromNative(nativeModifiers),
            )
        }
        desktop_gtk_h.NativeEvent_MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = NativeMouseMovedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseMovedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp.fromNative(NativeMouseMovedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_gtk_h.NativeEvent_MouseEntered() -> {
            val nativeEvent = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                windowId = NativeMouseEnteredEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseEnteredEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_gtk_h.NativeEvent_MouseExited() -> {
            val nativeEvent = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                windowId = NativeMouseExitedEvent.window_id(nativeEvent),
            )
        }
        desktop_gtk_h.NativeEvent_MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                windowId = NativeMouseUpEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseUpEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseUpEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp.fromNative(NativeMouseUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_gtk_h.NativeEvent_MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                windowId = NativeMouseDownEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseDownEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDownEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp.fromNative(NativeMouseDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_gtk_h.NativeEvent_NotificationClosed() -> {
            val nativeEvent = NativeEvent.notification_closed(s)
            Event.NotificationClosed(
                notificationId = NativeNotificationClosedEvent.notification_id(nativeEvent).toUInt(),
                action = fromOptionalNativeString(NativeNotificationClosedEvent.action(nativeEvent)),
                activationToken = fromOptionalNativeString(NativeNotificationClosedEvent.activation_token(nativeEvent)),
            )
        }
        desktop_gtk_h.NativeEvent_NotificationShown() -> {
            val nativeEvent = NativeEvent.notification_shown(s)
            val nativeNotificationId = NativeNotificationShownEvent.notification_id(nativeEvent)
            Event.NotificationShown(
                requestId = RequestId.fromNativeField(NativeNotificationShownEvent.request_id(nativeEvent)),
                notificationId = if (nativeNotificationId == 0) null else nativeNotificationId.toUInt(),
            )
        }
        desktop_gtk_h.NativeEvent_ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                windowId = NativeScrollWheelEvent.window_id(nativeEvent),
                scrollingDeltaX = NativeScrollWheelEvent.scroll_delta_x(nativeEvent).toFloat(),
                scrollingDeltaY = NativeScrollWheelEvent.scroll_delta_y(nativeEvent).toFloat(),
                timestamp = Timestamp.fromNative(NativeScrollWheelEvent.timestamp(nativeEvent)),
            )
        } desktop_gtk_h.NativeEvent_WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                windowId = NativeWindowScreenChangeEvent.window_id(nativeEvent),
                newScreenId = NativeWindowScreenChangeEvent.new_screen_id(nativeEvent).toULong(),
            )
        }
        desktop_gtk_h.NativeEvent_WindowClosed() -> {
            val nativeEvent = NativeEvent.window_closed(s)
            Event.WindowClosed(
                windowId = NativeWindowConfigureEvent.window_id(nativeEvent),
            )
        }
        desktop_gtk_h.NativeEvent_WindowConfigure() -> {
            val nativeEvent = NativeEvent.window_configure(s)
            Event.WindowConfigure(
                windowId = NativeWindowConfigureEvent.window_id(nativeEvent),
                size = LogicalSize.fromNative(NativeWindowConfigureEvent.size(nativeEvent)),
                active = NativeWindowConfigureEvent.active(nativeEvent),
                maximized = NativeWindowConfigureEvent.maximized(nativeEvent),
                fullscreen = NativeWindowConfigureEvent.fullscreen(nativeEvent),
                decorationMode = WindowDecorationMode.fromNative(NativeWindowConfigureEvent.decoration_mode(nativeEvent)),
                capabilities = WindowCapabilities.fromNative(NativeWindowConfigureEvent.capabilities(nativeEvent)),
            )
        }
        desktop_gtk_h.NativeEvent_WindowKeyboardEnter() -> {
            val nativeEvent = NativeEvent.window_keyboard_enter(s)
            Event.WindowKeyboardEnter(
                windowId = NativeWindowKeyboardEnterEvent.window_id(nativeEvent),
            )
        }
        desktop_gtk_h.NativeEvent_WindowKeyboardLeave() -> {
            val nativeEvent = NativeEvent.window_keyboard_leave(s)
            Event.WindowKeyboardLeave(windowId = NativeWindowKeyboardLeaveEvent.window_id(nativeEvent))
        }
        desktop_gtk_h.NativeEvent_ShouldRedraw() -> {
            val nativeEvent = NativeEvent.should_redraw(s)
            Event.ShouldRedraw(
                windowId = NativeShouldRedraw.window_id(nativeEvent),
            )
        }
        desktop_gtk_h.NativeEvent_ShouldRedrawDragIcon() -> {
            Event.ShouldRedrawDragIcon
        }
        desktop_gtk_h.NativeEvent_WindowDraw() -> {
            val nativeEvent = NativeEvent.window_draw(s)
            Event.WindowDraw(
                windowId = NativeWindowDrawEvent.window_id(nativeEvent),
                openGlDrawData = OpenGlDrawData.fromNative(NativeWindowDrawEvent.opengl_draw_data(nativeEvent)),
                size = PhysicalSize.fromNative(NativeWindowDrawEvent.physical_size(nativeEvent)),
            )
        }
        desktop_gtk_h.NativeEvent_WindowScaleChanged() -> {
            val nativeEvent = NativeEvent.window_scale_changed(s)
            Event.WindowScaleChanged(
                windowId = NativeWindowScaleChangedEvent.window_id(nativeEvent),
                newScale = NativeWindowScaleChangedEvent.new_scale(nativeEvent),
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}

internal fun FileDialog.CommonDialogParams.toNative(arena: Arena): MemorySegment {
    val result = NativeCommonFileDialogParams.allocate(arena)
    NativeCommonFileDialogParams.modal(result, modal)
    NativeCommonFileDialogParams.title(result, title?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
    NativeCommonFileDialogParams.accept_label(result, acceptLabel?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
    NativeCommonFileDialogParams.current_folder(result, currentFolder?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
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
        nameFieldStringValue?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
    )
    return result
}
