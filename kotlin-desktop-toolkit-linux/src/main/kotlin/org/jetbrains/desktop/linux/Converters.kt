package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.linux.generated.NativeColor
import org.jetbrains.desktop.linux.generated.NativeDataTransferAvailable
import org.jetbrains.desktop.linux.generated.NativeDataTransferContent
import org.jetbrains.desktop.linux.generated.NativeDragAndDropQueryData
import org.jetbrains.desktop.linux.generated.NativeEvent
import org.jetbrains.desktop.linux.generated.NativeKeyDownEvent
import org.jetbrains.desktop.linux.generated.NativeKeyModifiers
import org.jetbrains.desktop.linux.generated.NativeKeyUpEvent
import org.jetbrains.desktop.linux.generated.NativeLogicalPoint
import org.jetbrains.desktop.linux.generated.NativeLogicalRect
import org.jetbrains.desktop.linux.generated.NativeLogicalSize
import org.jetbrains.desktop.linux.generated.NativeModifiersChangedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseDownEvent
import org.jetbrains.desktop.linux.generated.NativeMouseDraggedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseEnteredEvent
import org.jetbrains.desktop.linux.generated.NativeMouseExitedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseMovedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseUpEvent
import org.jetbrains.desktop.linux.generated.NativePhysicalSize
import org.jetbrains.desktop.linux.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.linux.generated.NativeSoftwareDrawData
import org.jetbrains.desktop.linux.generated.NativeTextInputAvailabilityEvent
import org.jetbrains.desktop.linux.generated.NativeTextInputContext
import org.jetbrains.desktop.linux.generated.NativeTextInputDeleteSurroundingTextData
import org.jetbrains.desktop.linux.generated.NativeTextInputEvent
import org.jetbrains.desktop.linux.generated.NativeTextInputPreeditStringData
import org.jetbrains.desktop.linux.generated.NativeWindowCapabilities
import org.jetbrains.desktop.linux.generated.NativeWindowConfigureEvent
import org.jetbrains.desktop.linux.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.linux.generated.NativeWindowFocusChangeEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScreenChangeEvent
import org.jetbrains.desktop.linux.generated.NativeXdgDesktopSetting
import org.jetbrains.desktop.linux.generated.desktop_linux_h
import java.lang.foreign.Arena
import java.lang.foreign.MemoryLayout
import java.lang.foreign.MemorySegment
import kotlin.time.Duration.Companion.milliseconds
import kotlin.time.DurationUnit
import kotlin.time.toDuration

internal fun fromOptionalNativeString(s: MemorySegment): String? {
    return if (s == MemorySegment.NULL) null else s.getUtf8String(0)
}

/**
 * Converts UTF-8 offset to UTF-16 offset.
 */
public fun utf8OffsetToUtf16Offset(string: CharSequence, offset: Int): Int {
    if (offset == 0) {
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

internal fun LogicalRect.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalRect.allocate(arena)
    NativeLogicalRect.origin(result, point.toNative(arena))
    NativeLogicalRect.size(result, size.toNative(arena))
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
    maximize = NativeWindowCapabilities.maximixe(s),
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
            XdgDesktopSetting.TitlebarLayout(NativeXdgDesktopSetting.titlebar_layout(s).getUtf8String(0))
        }
        desktop_linux_h.NativeXdgDesktopSetting_DoubleClickIntervalMs() -> XdgDesktopSetting.DoubleClickInterval(
            value = NativeXdgDesktopSetting.double_click_interval_ms(s).milliseconds,
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
        desktop_linux_h.NativeXdgDesktopSetting_CursorSize() -> XdgDesktopSetting.CursorSize(NativeXdgDesktopSetting.cursor_size(s))
        desktop_linux_h.NativeXdgDesktopSetting_CursorTheme() -> XdgDesktopSetting.CursorTheme(
            NativeXdgDesktopSetting.cursor_theme(s).getUtf8String(0),
        )
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

internal fun TextInputPreeditStringData.Companion.fromNative(s: MemorySegment): TextInputPreeditStringData {
    return TextInputPreeditStringData(
        text = fromOptionalNativeString(NativeTextInputPreeditStringData.text(s)),
        cursorBeginBytePos = NativeTextInputPreeditStringData.cursor_begin_byte_pos(s),
        cursorEndBytePos = NativeTextInputPreeditStringData.cursor_end_byte_pos(s),
    )
}

internal fun TextInputDeleteSurroundingTextData.Companion.fromNative(s: MemorySegment): TextInputDeleteSurroundingTextData {
    return TextInputDeleteSurroundingTextData(
        beforeLengthInBytes = NativeTextInputDeleteSurroundingTextData.before_length_in_bytes(s),
        afterLengthInBytes = NativeTextInputDeleteSurroundingTextData.after_length_in_bytes(s),
    )
}

internal fun TextInputContext.toNative(arena: Arena): MemorySegment {
    val result = NativeTextInputContext.allocate(arena)
    NativeTextInputContext.surrounding_text(result, arena.allocateUtf8String(surroundingText))
    NativeTextInputContext.cursor_codepoint_offset(result, cursorCodepointOffset)
    NativeTextInputContext.selection_start_codepoint_offset(result, selectionStartCodepointOffset)
    NativeTextInputContext.is_multiline(result, isMultiline)
    NativeTextInputContext.content_purpose(result, contentPurpose.toNative())
    NativeTextInputContext.cursor_rectangle(result, cursorRectangle.toNative(arena))
    NativeTextInputContext.change_caused_by_input_method(result, changeCausedByInputMethod)
    return result
}

internal fun DataTransferContent.Companion.fromNative(s: MemorySegment): DataTransferContent {
    val mimeTypesString = NativeDataTransferContent.mime_types(s).getUtf8String(0)
    val nativeU8Array = NativeDataTransferContent.data(s)
    val len = NativeBorrowedArray_u8.len(nativeU8Array)
    val buf = ByteArray(len.toInt())
    val dataPtr = NativeBorrowedArray_u8.ptr(nativeU8Array)
    for (i in 0 until len) {
        buf[i.toInt()] = dataPtr.getAtIndex(desktop_linux_h.C_CHAR, i)
    }
    return DataTransferContent(
        data = buf,
        mimeTypes = mimeTypesString.split(','),
    )
}

internal fun DataSource.Companion.fromNative(nativeDataSource: Int): DataSource = when (nativeDataSource) {
    desktop_linux_h.NativeDataSource_Clipboard() -> DataSource.Clipboard
    desktop_linux_h.NativeDataSource_DragAndDrop() -> DataSource.DragAndDrop
    else -> error("Unexpected data source type $nativeDataSource")
}

internal fun mimeTypesToNative(arena: Arena, mimeTypes: List<String>): MemorySegment {
    return arena.allocateUtf8String(mimeTypes.joinToString(","))
}

internal fun ByteArray?.toNative(arena: Arena): MemorySegment {
    val nativeDataArray = NativeBorrowedArray_u8.allocate(arena)
    if (this == null) {
        NativeBorrowedArray_u8.len(nativeDataArray, 0)
        NativeBorrowedArray_u8.ptr(nativeDataArray, MemorySegment.NULL)
    } else {
        NativeBorrowedArray_u8.len(nativeDataArray, size.toLong())
        val nativeArray = arena.allocate(MemoryLayout.sequenceLayout(size.toLong(), desktop_linux_h.C_CHAR))
        this.forEachIndexed { i, b ->
            nativeArray.setAtIndex(desktop_linux_h.C_CHAR, i.toLong(), b)
        }

        NativeBorrowedArray_u8.ptr(nativeDataArray, nativeArray)

        NativeBorrowedArray_u8.deinit(
            nativeDataArray,
            NativeBorrowedArray_u8.deinit.allocate({ ptr, len ->
                arena.close()
            }, arena),
        )
    }

    return nativeDataArray
}

internal fun DragAndDropQueryData.Companion.fromNative(s: MemorySegment): DragAndDropQueryData {
    return DragAndDropQueryData(
        windowId = NativeDragAndDropQueryData.window_id(s),
        point = LogicalPoint.fromNative(NativeDragAndDropQueryData.point(s)),
    )
}

internal fun DragAction.toNative(): Int = when (this) {
    DragAction.Copy -> desktop_linux_h.NativeDragAction_Copy()
    DragAction.Move -> desktop_linux_h.NativeDragAction_Move()
    DragAction.Ask -> desktop_linux_h.NativeDragAction_Ask()
}

internal fun Event.Companion.fromNative(s: MemorySegment): Event {
    return when (NativeEvent.tag(s)) {
        desktop_linux_h.NativeEvent_DataTransferAvailable() -> {
            val nativeEvent = NativeEvent.data_transfer_available(s)
            val mimeTypesString = NativeDataTransferAvailable.mime_types(nativeEvent).getUtf8String(0)
            Event.DataTransferAvailable(mimeTypes = mimeTypesString.split(","))
        }
        desktop_linux_h.NativeEvent_DataTransfer() -> {
            val nativeEvent = NativeEvent.data_transfer(s)
            Event.DataTransfer(
                serial = NativeDataTransferContent.serial(nativeEvent),
                data = DataTransferContent.fromNative(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            Event.KeyDown(
                keyCode = KeyCode(NativeKeyDownEvent.code(nativeEvent)),
                characters = fromOptionalNativeString(NativeKeyDownEvent.characters(nativeEvent)),
                key = KeySym(NativeKeyDownEvent.key(nativeEvent)),
                isRepeat = NativeKeyDownEvent.is_repeat(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                characters = fromOptionalNativeString(NativeKeyUpEvent.characters(nativeEvent)),
                key = KeySym(NativeKeyUpEvent.key(nativeEvent)),
                keyCode = KeyCode(NativeKeyUpEvent.code(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_TextInputAvailability() -> {
            val nativeEvent = NativeEvent.text_input(s)
            Event.TextInputAvailability(NativeTextInputAvailabilityEvent.available(nativeEvent))
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
        desktop_linux_h.NativeEvent_ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            Event.ModifiersChanged(
                modifiers = KeyModifiers.fromNative(NativeModifiersChangedEvent.modifiers(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                locationInWindow = LogicalPoint.fromNative(NativeMouseMovedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseMovedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseDragged() -> {
            val nativeEvent = NativeEvent.mouse_dragged(s)
            Event.MouseDragged(
                button = MouseButton(NativeMouseDraggedEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDraggedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseDraggedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseEntered() -> {
            val nativeEvent = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                locationInWindow = LogicalPoint.fromNative(NativeMouseEnteredEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseExited() -> {
            val nativeEvent = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                locationInWindow = LogicalPoint.fromNative(NativeMouseExitedEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                button = MouseButton(NativeMouseUpEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseUpEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                button = MouseButton(NativeMouseDownEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDownEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                scrollingDeltaX = NativeScrollWheelEvent.scrolling_delta_x(nativeEvent).toFloat(),
                scrollingDeltaY = NativeScrollWheelEvent.scrolling_delta_y(nativeEvent).toFloat(),
                locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeScrollWheelEvent.timestamp(nativeEvent)),
            )
        } desktop_linux_h.NativeEvent_WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                newScreenId = NativeWindowScreenChangeEvent.new_screen_id(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_WindowConfigure() -> {
            val nativeEvent = NativeEvent.window_configure(s)
            Event.WindowConfigure(
                size = LogicalSize.fromNative(NativeWindowConfigureEvent.size(nativeEvent)),
                active = NativeWindowConfigureEvent.active(nativeEvent),
                maximized = NativeWindowConfigureEvent.maximized(nativeEvent),
                fullscreen = NativeWindowConfigureEvent.fullscreen(nativeEvent),
                clientSideDecorations = NativeWindowConfigureEvent.client_side_decorations(nativeEvent),
                capabilities = WindowCapabilities.fromNative(NativeWindowConfigureEvent.capabilities(nativeEvent)),
            )
        }
        desktop_linux_h.NativeEvent_WindowFocusChange() -> {
            val nativeEvent = NativeEvent.window_focus_change(s)
            Event.WindowFocusChange(
                isKeyWindow = NativeWindowFocusChangeEvent.is_key(nativeEvent),
                isMainWindow = NativeWindowFocusChangeEvent.is_main(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_WindowCloseRequest() -> {
            Event.WindowCloseRequest
        }
        desktop_linux_h.NativeEvent_WindowDraw() -> {
            val nativeEvent = NativeEvent.window_draw(s)
            Event.WindowDraw(
                softwareDrawData = SoftwareDrawData.fromNative(NativeWindowDrawEvent.software_draw_data(nativeEvent)),
                size = PhysicalSize.fromNative(NativeWindowDrawEvent.physical_size(nativeEvent)),
                scale = NativeWindowDrawEvent.scale(nativeEvent),
            )
        }
        desktop_linux_h.NativeEvent_WindowScaleChanged() -> {
            val nativeEvent = NativeEvent.window_scale_changed(s)
            Event.WindowScaleChanged(
                newScale = NativeWindowScaleChangedEvent.new_scale(nativeEvent),
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}
