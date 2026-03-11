package org.jetbrains.desktop.gtk

import org.jetbrains.desktop.gtk.generated.NativeBorrowedArray_FfiSupportedActionsForMime
import org.jetbrains.desktop.gtk.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.gtk.generated.NativeCommonFileDialogParams
import org.jetbrains.desktop.gtk.generated.NativeDataTransferAvailableEvent
import org.jetbrains.desktop.gtk.generated.NativeDataTransferCancelledEvent
import org.jetbrains.desktop.gtk.generated.NativeDataTransferContent
import org.jetbrains.desktop.gtk.generated.NativeDataTransferEvent
import org.jetbrains.desktop.gtk.generated.NativeDragAndDropFeedbackFinishedEvent
import org.jetbrains.desktop.gtk.generated.NativeDragAndDropFinishedEvent
import org.jetbrains.desktop.gtk.generated.NativeDragAndDropLeaveEvent
import org.jetbrains.desktop.gtk.generated.NativeDragAndDropQueryData
import org.jetbrains.desktop.gtk.generated.NativeDragIconDrawEvent
import org.jetbrains.desktop.gtk.generated.NativeDropPerformedEvent
import org.jetbrains.desktop.gtk.generated.NativeEvent
import org.jetbrains.desktop.gtk.generated.NativeFfiDesktopSetting
import org.jetbrains.desktop.gtk.generated.NativeFfiDragAndDropQueryResponse
import org.jetbrains.desktop.gtk.generated.NativeFfiSupportedActionsForMime
import org.jetbrains.desktop.gtk.generated.NativeFfiTextInputSurroundingText
import org.jetbrains.desktop.gtk.generated.NativeFfiTransferDataResponse
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
import org.jetbrains.desktop.gtk.generated.NativeScreenInfoArray
import org.jetbrains.desktop.gtk.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.gtk.generated.NativeTextInputContext
import org.jetbrains.desktop.gtk.generated.NativeTextInputDeleteSurroundingTextData
import org.jetbrains.desktop.gtk.generated.NativeTextInputEvent
import org.jetbrains.desktop.gtk.generated.NativeTextInputPreeditAttribute
import org.jetbrains.desktop.gtk.generated.NativeTextInputPreeditStringData
import org.jetbrains.desktop.gtk.generated.NativeWindowConfigureEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowDecorationMode
import org.jetbrains.desktop.gtk.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowFrameTick
import org.jetbrains.desktop.gtk.generated.NativeWindowKeyboardEnterEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowKeyboardLeaveEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.gtk.generated.NativeWindowScreenChangeEvent
import org.jetbrains.desktop.gtk.generated.desktop_gtk_h
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
    RenderingMode.GL -> desktop_gtk_h.NativeRenderingMode_Gl()
    RenderingMode.GL_ES -> desktop_gtk_h.NativeRenderingMode_GlEs()
}

private fun WindowDecorationMode.Companion.fromNative(native: MemorySegment): WindowDecorationMode {
    return when (val nativeTag = NativeWindowDecorationMode.tag(native)) {
        desktop_gtk_h.NativeWindowDecorationMode_CustomTitlebar() -> WindowDecorationMode.CustomTitlebar(
            NativeWindowDecorationMode.custom_titlebar(
                native,
            ),
        )

        desktop_gtk_h.NativeWindowDecorationMode_Server() -> WindowDecorationMode.Server
        else -> error("Unexpected Window decoration mode: $nativeTag")
    }
}

internal fun WindowDecorationMode.toNative(arena: Arena): MemorySegment {
    val result = NativeWindowDecorationMode.allocate(arena)
    when (this) {
        WindowDecorationMode.Server -> NativeWindowDecorationMode.tag(result, desktop_gtk_h.NativeWindowDecorationMode_Server())
        is WindowDecorationMode.CustomTitlebar -> {
            NativeWindowDecorationMode.tag(result, desktop_gtk_h.NativeWindowDecorationMode_CustomTitlebar())
            NativeWindowDecorationMode.custom_titlebar(result, height)
        }
    }
    return result
}

internal fun DesktopSetting.Companion.fromNative(s: MemorySegment): DesktopSetting {
    return when (val nativeTag = NativeFfiDesktopSetting.tag(s)) {
        desktop_gtk_h.NativeFfiDesktopSetting_DoubleClickIntervalMs() -> DesktopSetting.DoubleClickInterval(
            value = NativeFfiDesktopSetting.double_click_interval_ms(s).milliseconds,
        )
        desktop_gtk_h.NativeFfiDesktopSetting_DoubleClickDistancePixels() -> DesktopSetting.DoubleClickDistancePixels(
            value = NativeFfiDesktopSetting.double_click_distance_pixels(s),
        )

        desktop_gtk_h.NativeFfiDesktopSetting_DragAndDropDragThresholdPixels() -> DesktopSetting.DragAndDropDragThresholdPixels(
            value = NativeFfiDesktopSetting.drag_and_drop_drag_threshold_pixels(s),
        )

        desktop_gtk_h.NativeFfiDesktopSetting_ColorScheme() -> DesktopSetting.ColorScheme(
            when (val raw = NativeFfiDesktopSetting.color_scheme(s)) {
                desktop_gtk_h.NativeXdgDesktopColorScheme_NoPreference() -> ColorSchemeValue.NoPreference
                desktop_gtk_h.NativeXdgDesktopColorScheme_PreferDark() -> ColorSchemeValue.PreferDark
                desktop_gtk_h.NativeXdgDesktopColorScheme_PreferLight() -> ColorSchemeValue.PreferLight
                else -> error("Unexpected color scheme $raw")
            },
        )

        desktop_gtk_h.NativeFfiDesktopSetting_AccentColor() -> DesktopSetting.AccentColor(
            when (val raw = NativeFfiDesktopSetting.accent_color(s)) {
                desktop_gtk_h.NativeAccentColor_Unknown() -> AccentColorValue.Unknown
                desktop_gtk_h.NativeAccentColor_Blue() -> AccentColorValue.Blue
                desktop_gtk_h.NativeAccentColor_Teal() -> AccentColorValue.Teal
                desktop_gtk_h.NativeAccentColor_Green() -> AccentColorValue.Green
                desktop_gtk_h.NativeAccentColor_Yellow() -> AccentColorValue.Yellow
                desktop_gtk_h.NativeAccentColor_Orange() -> AccentColorValue.Orange
                desktop_gtk_h.NativeAccentColor_Red() -> AccentColorValue.Red
                desktop_gtk_h.NativeAccentColor_Pink() -> AccentColorValue.Pink
                desktop_gtk_h.NativeAccentColor_Purple() -> AccentColorValue.Purple
                desktop_gtk_h.NativeAccentColor_Slate() -> AccentColorValue.Slate
                desktop_gtk_h.NativeAccentColor_Red() -> AccentColorValue.Red
                else -> error("Unexpected accent color $raw")
            },
        )

        desktop_gtk_h.NativeFfiDesktopSetting_FontHinting() -> DesktopSetting.FontHinting(
            when (NativeFfiDesktopSetting.font_hinting(s)) {
                desktop_gtk_h.NativeFontHinting_None() -> FontHintingValue.None
                desktop_gtk_h.NativeFontHinting_Slight() -> FontHintingValue.Slight
                desktop_gtk_h.NativeFontHinting_Medium() -> FontHintingValue.Medium
                desktop_gtk_h.NativeFontHinting_Full() -> FontHintingValue.Full
                else -> error("Unexpected font hinting ${NativeFfiDesktopSetting.font_hinting(s)}")
            },
        )

        desktop_gtk_h.NativeFfiDesktopSetting_FontRgbaOrder() -> DesktopSetting.FontRgbaOrder(
            when (NativeFfiDesktopSetting.font_rgba_order(s)) {
                desktop_gtk_h.NativeFontRgbaOrder_None() -> FontRgbaOrderValue.None
                desktop_gtk_h.NativeFontRgbaOrder_Rgb() -> FontRgbaOrderValue.Rgb
                desktop_gtk_h.NativeFontRgbaOrder_Bgr() -> FontRgbaOrderValue.Bgr
                desktop_gtk_h.NativeFontRgbaOrder_Vrgb() -> FontRgbaOrderValue.Vrgb
                desktop_gtk_h.NativeFontRgbaOrder_Vbgr() -> FontRgbaOrderValue.Vbgr
                else -> error("Unexpected font rgba order ${NativeFfiDesktopSetting.font_rgba_order(s)}")
            },
        )

        desktop_gtk_h.NativeFfiDesktopSetting_CursorBlink() -> DesktopSetting.CursorBlink(NativeFfiDesktopSetting.cursor_blink(s))

        desktop_gtk_h.NativeFfiDesktopSetting_CursorBlinkTimeMs() -> DesktopSetting.CursorBlinkTime(
            NativeFfiDesktopSetting.cursor_blink_time_ms(s).toDuration(DurationUnit.MILLISECONDS),
        )

        desktop_gtk_h.NativeFfiDesktopSetting_CursorBlinkTimeoutSeconds() -> DesktopSetting.CursorBlinkTimeout(
            NativeFfiDesktopSetting.cursor_blink_timeout_seconds(s).toDuration(DurationUnit.SECONDS),
        )

        desktop_gtk_h.NativeFfiDesktopSetting_OverlayScrolling() -> DesktopSetting.OverlayScrolling(
            NativeFfiDesktopSetting.overlay_scrolling(s),
        )

        desktop_gtk_h.NativeFfiDesktopSetting_AudibleBell() -> DesktopSetting.AudibleBell(NativeFfiDesktopSetting.audible_bell(s))
        desktop_gtk_h.NativeFfiDesktopSetting_MiddleClickPaste() -> DesktopSetting.MiddleClickPaste(
            NativeFfiDesktopSetting.middle_click_paste(s),
        )

        desktop_gtk_h.NativeFfiDesktopSetting_IsComposited() -> DesktopSetting.IsComposited(
            NativeFfiDesktopSetting.is_composited(s),
        )

        desktop_gtk_h.NativeFfiDesktopSetting_EnableAnimations() -> DesktopSetting.EnableAnimations(
            NativeFfiDesktopSetting.enable_animations(s),
        )
        desktop_gtk_h.NativeFfiDesktopSetting_PrimaryButtonWarpsSlider() -> DesktopSetting.PrimaryButtonWarpsSlider(
            NativeFfiDesktopSetting.primary_button_warps_slider(s),
        )
        desktop_gtk_h.NativeFfiDesktopSetting_RecentFilesEnabled() -> DesktopSetting.RecentFilesEnabled(
            NativeFfiDesktopSetting.recent_files_enabled(s),
        )
        desktop_gtk_h.NativeFfiDesktopSetting_RecentFilesMaxAgeDays() -> DesktopSetting.RecentFilesMaxAgeDays(
            NativeFfiDesktopSetting.recent_files_max_age_days(s),
        )
        else -> error("Unexpected setting $nativeTag")
    }
}

internal fun OpenGlDrawData.Companion.fromNative(nativeSoftwareDrawData: MemorySegment): OpenGlDrawData {
    val framebuffer = NativeOpenGlDrawData.framebuffer(nativeSoftwareDrawData)
    val isEs = NativeOpenGlDrawData.is_es(nativeSoftwareDrawData)

    return OpenGlDrawData(framebuffer = framebuffer, isEs = isEs)
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
            TextInputContextHint.Spellcheck -> desktop_gtk_h.NativeTextInputContextHint_Spellcheck()
            TextInputContextHint.NoSpellcheck -> desktop_gtk_h.NativeTextInputContextHint_NoSpellcheck()
            TextInputContextHint.WordCompletion -> desktop_gtk_h.NativeTextInputContextHint_WordCompletion()
            TextInputContextHint.Lowercase -> desktop_gtk_h.NativeTextInputContextHint_Lowercase()
            TextInputContextHint.UppercaseChars -> desktop_gtk_h.NativeTextInputContextHint_UppercaseChars()
            TextInputContextHint.UppercaseWords -> desktop_gtk_h.NativeTextInputContextHint_UppercaseWords()
            TextInputContextHint.UppercaseSentences -> desktop_gtk_h.NativeTextInputContextHint_UppercaseSentences()
            TextInputContextHint.InhibitOsk -> desktop_gtk_h.NativeTextInputContextHint_InhibitOsk()
            TextInputContextHint.VerticalWriting -> desktop_gtk_h.NativeTextInputContextHint_VerticalWriting()
            TextInputContextHint.Emoji -> desktop_gtk_h.NativeTextInputContextHint_Emoji()
            TextInputContextHint.NoEmoji -> desktop_gtk_h.NativeTextInputContextHint_NoEmoji()
            TextInputContextHint.Private -> desktop_gtk_h.NativeTextInputContextHint_Private()
        }
    }
    return nativeHints
}

internal fun TextInputPreeditUnderlineType.Companion.fromNative(raw: Int): TextInputPreeditUnderlineType {
    return when (raw) {
        desktop_gtk_h.NativeTextInputPreeditUnderlineType_None() -> TextInputPreeditUnderlineType.None
        desktop_gtk_h.NativeTextInputPreeditUnderlineType_Single() -> TextInputPreeditUnderlineType.Single
        desktop_gtk_h.NativeTextInputPreeditUnderlineType_Double() -> TextInputPreeditUnderlineType.Double
        desktop_gtk_h.NativeTextInputPreeditUnderlineType_Low() -> TextInputPreeditUnderlineType.Low
        desktop_gtk_h.NativeTextInputPreeditUnderlineType_Error() -> TextInputPreeditUnderlineType.Error
        else -> error("Unexpected TextInputPreeditUnderlineType: $raw")
    }
}

internal fun TextInputPreeditAttribute.Companion.fromNative(s: MemorySegment): TextInputPreeditAttribute {
    return TextInputPreeditAttribute(
        beginBytePos = NativeTextInputPreeditAttribute.begin_byte_pos(s).toUInt(),
        endBytePos = NativeTextInputPreeditAttribute.end_byte_pos(s).toUInt(),
        underline = TextInputPreeditUnderlineType.fromNative(NativeTextInputPreeditAttribute.underline(s)),
        foregroundHighlight = NativeTextInputPreeditAttribute.foreground_highlight(s),
        backgroundHighlight = NativeTextInputPreeditAttribute.background_highlight(s),
        strikethrough = NativeTextInputPreeditAttribute.strikethrough(s),
        bold = NativeTextInputPreeditAttribute.bold(s),
        italic = NativeTextInputPreeditAttribute.italic(s),
    )
}

internal fun TextInputPreeditStringData.Companion.fromNative(s: MemorySegment): TextInputPreeditStringData {
    val rawAttributes = NativeTextInputPreeditStringData.attributes(s)
    val attributes = mutableListOf<TextInputPreeditAttribute>()
    if (rawAttributes != MemorySegment.NULL) {
        val ptr = NativeScreenInfoArray.ptr(rawAttributes)
        val len = NativeScreenInfoArray.len(rawAttributes)

        for (i in 0 until len) {
            attributes.add(TextInputPreeditAttribute.fromNative(NativeTextInputPreeditAttribute.asSlice(ptr, i)))
        }
    }

    return TextInputPreeditStringData(
        text = fromOptionalNativeString(NativeTextInputPreeditStringData.text(s)),
        cursorBytePos = NativeTextInputPreeditStringData.cursor_byte_pos(s),
        attributes = attributes,
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
    NativeTextInputContext.hints(result, hints.toNative())
    NativeTextInputContext.content_purpose(result, contentPurpose.toNative())
    NativeTextInputContext.cursor_rectangle(result, cursorRectangle.toNative(arena))
    return result
}

internal fun TextInputSurroundingText?.toNative(arena: Arena, objId: Long): MemorySegment {
    val result = NativeFfiTextInputSurroundingText.allocate(arena)
    if (this == null) {
        val text: ByteArray? = null
        NativeFfiTextInputSurroundingText.obj_id(result, 0)
        NativeFfiTextInputSurroundingText.surrounding_text(result, text.toNative(arena))
        NativeFfiTextInputSurroundingText.cursor_codepoint_offset(result, 0)
        NativeFfiTextInputSurroundingText.selection_start_codepoint_offset(result, 0)
    } else {
        NativeFfiTextInputSurroundingText.obj_id(result, objId)
        NativeFfiTextInputSurroundingText.surrounding_text(result, surroundingText.encodeToByteArray().toNative(arena))
        NativeFfiTextInputSurroundingText.cursor_codepoint_offset(result, cursorCodepointOffset.toShort())
        NativeFfiTextInputSurroundingText.selection_start_codepoint_offset(result, selectionStartCodepointOffset.toShort())
    }
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

private fun ByteArray?.toNative(arena: Arena): MemorySegment {
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
    }

    return nativeDataArray
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
    NativeFfiSupportedActionsForMime.supported_mime_type(result, arena.allocateUtf8String(supportedMimeType))
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
        desktop_gtk_h.NativeEvent_DesktopSettingChange() -> {
            val nativeEvent = NativeEvent.desktop_setting_change(s)
            Event.DesktopSettingChange(
                setting = DesktopSetting.fromNative(nativeEvent),
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
                locationInWindow = LogicalPoint.fromNative(NativeDropPerformedEvent.location_in_window(nativeEvent)),
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
        desktop_gtk_h.NativeEvent_DragAndDropFeedbackFinished() -> {
            val nativeEvent = NativeEvent.drag_and_drop_feedback_finished(s)
            Event.DragAndDropFeedbackFinished(
                windowId = NativeDragAndDropFeedbackFinishedEvent.window_id(nativeEvent),
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
                insetStart = LogicalSize.fromNative(NativeWindowConfigureEvent.inset_start(nativeEvent)),
                insetEnd = LogicalSize.fromNative(NativeWindowConfigureEvent.inset_end(nativeEvent)),
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
        desktop_gtk_h.NativeEvent_WindowFrameTick() -> {
            val nativeEvent = NativeEvent.window_frame_tick(s)
            Event.WindowFrameTick(
                windowId = NativeWindowFrameTick.window_id(nativeEvent),
                frameTimeMicroseconds = NativeWindowFrameTick.frame_time_microseconds(nativeEvent),
            )
        }
        desktop_gtk_h.NativeEvent_DragIconFrameTick() -> {
            Event.DragIconFrameTick
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
