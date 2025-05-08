package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeAutoDropArray_WindowButtonType
import org.jetbrains.desktop.linux.generated.NativeEvent
import org.jetbrains.desktop.linux.generated.NativeKeyDownEvent
import org.jetbrains.desktop.linux.generated.NativeKeyUpEvent
import org.jetbrains.desktop.linux.generated.NativeModifiersChangedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseDownEvent
import org.jetbrains.desktop.linux.generated.NativeMouseDraggedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseEnteredEvent
import org.jetbrains.desktop.linux.generated.NativeMouseExitedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseMovedEvent
import org.jetbrains.desktop.linux.generated.NativeMouseUpEvent
import org.jetbrains.desktop.linux.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.linux.generated.NativeTextInputAvailabilityEvent
import org.jetbrains.desktop.linux.generated.NativeTextInputEvent
import org.jetbrains.desktop.linux.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.linux.generated.NativeWindowFocusChangeEvent
import org.jetbrains.desktop.linux.generated.NativeWindowFullScreenToggleEvent
import org.jetbrains.desktop.linux.generated.NativeWindowResizeEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScreenChangeEvent
import java.lang.foreign.MemorySegment
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds
import org.jetbrains.desktop.linux.generated.desktop_linux_h as desktop_h

@JvmInline
public value class Timestamp(
    // Count of milliseconds passed since some fixed but arbitrary moment in the past
    private val value: Int,
) {
    public fun toDuration(): Duration {
        return value.milliseconds
    }
}

public enum class WindowButtonType {
    AppMenu,
    Icon,
    Spacer,
    Title,
    Minimize,
    Maximize,
    Close,
    ;

    internal companion object {
        fun fromNative(s: MemorySegment, index: Long): WindowButtonType {
            val v = s.getAtIndex(desktop_h.NativeWindowButtonType, index)
            return when (v) {
                desktop_h.NativeWindowButtonType_AppMenu() -> AppMenu
                desktop_h.NativeWindowButtonType_Icon() -> Icon
                desktop_h.NativeWindowButtonType_Spacer() -> Spacer
                desktop_h.NativeWindowButtonType_Minimize() -> Minimize
                desktop_h.NativeWindowButtonType_Maximize() -> Maximize
                desktop_h.NativeWindowButtonType_Close() -> Close
                else -> error("Unexpected WindowButtonType tag $v")
            }
        }

        fun fromNativeArray(nativeArray: MemorySegment): List<WindowButtonType> {
            val ptr = NativeAutoDropArray_WindowButtonType.ptr(nativeArray)
            val len = NativeAutoDropArray_WindowButtonType.len(nativeArray)

            return (0 until len).map {
                fromNative(ptr, it)
            }
        }
    }
}

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
    public data class TitlebarLayout(val layoutLeft: List<WindowButtonType>, val layoutRight: List<WindowButtonType>) : XdgDesktopSetting()

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

public data class WindowCapabilities(
    /** `show_window_menu` is available. */
    public val windowMenu: Boolean,

    /** Window can be maximized and unmaximized. */
    public val maximixe: Boolean,

    /** Window can be fullscreened and unfullscreened. */
    public val fullscreen: Boolean,

    /** Window can be minimized. */
    public val minimize: Boolean,
) {
    internal companion object;
}

public data class SoftwareDrawData(val canvas: Long, val stride: Int) {
    internal companion object;
}

public class ClipboardData(public val data: ByteArray, public val mimeTypes: List<String>) {
    internal companion object;
}

public data class DragAndDropQueryData(public val windowId: WindowId, public val point: LogicalPoint) {
    internal companion object;
}

public sealed class Event {
    public companion object {
//        public fun pressedMouseButtons(): MouseButtonsSet {
//            return MouseButtonsSet(ffiDownCall { desktop_h.events_pressed_mouse_buttons() })
//        }
//
//        public fun pressedModifiers(): KeyModifiersSet {
//            return KeyModifiersSet(ffiDownCall { desktop_h.events_pressed_modifiers() })
//        }
//
//        public fun cursorLocationInScreen(): LogicalPoint {
//            return Arena.ofConfined().use { arena ->
//                LogicalPoint.fromNative(ffiDownCall { desktop_h.events_cursor_location_in_screen(arena) })
//            }
//        }
    }

    public data class ClipboardPaste(
        val data: ClipboardData,
    ) : Event()

    public data class KeyDown(
        val keyCode: KeyCode,
        val characters: String?,
        val key: KeySym,
        val isRepeat: Boolean,
        val timestamp: Timestamp,
    ) : Event()

    public data class KeyUp(
        val keyCode: KeyCode,
        val characters: String?,
        val key: KeySym,
        val timestamp: Timestamp,
    ) : Event()

    public data class TextInputAvailability(val available: Boolean) : Event()

    public data class TextInput(
        val preeditStringData: TextInputPreeditStringData?,
        val commitStringData: TextInputCommitStringData?,
        val deleteSurroundingTextData: TextInputDeleteSurroundingTextData?,
    ) : Event()

    public data class ComposedTextChanged(
        val text: String?,
        val cursorBegin: Int,
        val cursorEnd: Int,
    ) : Event()

    public data class ModifiersChanged(
        val modifiers: KeyModifiers,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseMoved(
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseDragged(
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseEntered(
        val locationInWindow: LogicalPoint,
    ) : Event()

    public data class MouseExited(
        val locationInWindow: LogicalPoint,
    ) : Event()

    public data class MouseUp(
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseDown(
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
        private val native: MemorySegment,
    ) : Event()

    public data class ScrollWheel(
        val scrollingDeltaX: LogicalPixels,
        val scrollingDeltaY: LogicalPixels,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class WindowScreenChange(
        val newScreenId: ScreenId,
    ) : Event()

    public data class WindowResize(
        val size: LogicalSize,
        val active: Boolean,
        val maximized: Boolean,
        val fullscreen: Boolean,
        val clientSideDecorations: Boolean,
        val capabilities: WindowCapabilities,
    ) : Event()

    public data class WindowFocusChange(
        val isKeyWindow: Boolean,
        val isMainWindow: Boolean,
    ) : Event()

    public data class WindowFullScreenToggle(
        val isFullScreen: Boolean,
    ) : Event()

    public data class WindowDraw(
        val softwareDrawData: SoftwareDrawData?,
        val size: PhysicalSize,
        val scale: Double,
    ) : Event()

    public data class WindowScaleChanged(
        val newScale: Double,
    ) : Event()

    public data object WindowCloseRequest : Event()
}

internal fun Event.Companion.fromNative(s: MemorySegment): Event {
    return when (NativeEvent.tag(s)) {
        desktop_h.NativeEvent_ClipboardPaste() -> {
            val nativeEvent = NativeEvent.clipboard_paste(s)
            Event.ClipboardPaste(data = ClipboardData.fromNative(nativeEvent))
        }
        desktop_h.NativeEvent_KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            Event.KeyDown(
                keyCode = KeyCode(NativeKeyDownEvent.code(nativeEvent)),
                characters = fromOptionalNativeString(NativeKeyDownEvent.characters(nativeEvent)),
                key = KeySym(NativeKeyDownEvent.key(nativeEvent)),
                isRepeat = NativeKeyDownEvent.is_repeat(nativeEvent),
                timestamp = Timestamp(NativeKeyDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                characters = fromOptionalNativeString(NativeKeyUpEvent.characters(nativeEvent)),
                key = KeySym(NativeKeyUpEvent.key(nativeEvent)),
                keyCode = KeyCode(NativeKeyUpEvent.code(nativeEvent)),
                timestamp = Timestamp(NativeKeyUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_TextInputAvailability() -> {
            val nativeEvent = NativeEvent.text_input(s)
            Event.TextInputAvailability(NativeTextInputAvailabilityEvent.available(nativeEvent))
        }
        desktop_h.NativeEvent_TextInput() -> {
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
        desktop_h.NativeEvent_ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            Event.ModifiersChanged(
                modifiers = KeyModifiers.fromNative(NativeModifiersChangedEvent.modifiers(nativeEvent)),
                timestamp = Timestamp(NativeModifiersChangedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                locationInWindow = LogicalPoint.fromNative(NativeMouseMovedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseMovedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_MouseDragged() -> {
            val nativeEvent = NativeEvent.mouse_dragged(s)
            Event.MouseDragged(
                button = MouseButton(NativeMouseDraggedEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDraggedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseDraggedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_MouseEntered() -> {
            val nativeEvent = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                locationInWindow = LogicalPoint.fromNative(NativeMouseEnteredEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_MouseExited() -> {
            val nativeEvent = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                locationInWindow = LogicalPoint.fromNative(NativeMouseExitedEvent.location_in_window(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                button = MouseButton(NativeMouseUpEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseUpEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                button = MouseButton(NativeMouseDownEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDownEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseDownEvent.timestamp(nativeEvent)),
                native = nativeEvent,
            )
        }
        desktop_h.NativeEvent_ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                scrollingDeltaX = NativeScrollWheelEvent.scrolling_delta_x(nativeEvent).toFloat(),
                scrollingDeltaY = NativeScrollWheelEvent.scrolling_delta_y(nativeEvent).toFloat(),
                locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeScrollWheelEvent.timestamp(nativeEvent)),
            )
        } desktop_h.NativeEvent_WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                newScreenId = NativeWindowScreenChangeEvent.new_screen_id(nativeEvent),
            )
        }
        desktop_h.NativeEvent_WindowResize() -> {
            val nativeEvent = NativeEvent.window_resize(s)
            Event.WindowResize(
                size = LogicalSize.fromNative(NativeWindowResizeEvent.size(nativeEvent)),
                active = NativeWindowResizeEvent.active(nativeEvent),
                maximized = NativeWindowResizeEvent.maximized(nativeEvent),
                fullscreen = NativeWindowResizeEvent.fullscreen(nativeEvent),
                clientSideDecorations = NativeWindowResizeEvent.client_side_decorations(nativeEvent),
                capabilities = WindowCapabilities.fromNative(NativeWindowResizeEvent.capabilities(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_WindowFocusChange() -> {
            val nativeEvent = NativeEvent.window_focus_change(s)
            Event.WindowFocusChange(
                isKeyWindow = NativeWindowFocusChangeEvent.is_key(nativeEvent),
                isMainWindow = NativeWindowFocusChangeEvent.is_main(nativeEvent),
            )
        }
        desktop_h.NativeEvent_WindowCloseRequest() -> {
            Event.WindowCloseRequest
        }
        desktop_h.NativeEvent_WindowFullScreenToggle() -> {
            val nativeEvent = NativeEvent.window_full_screen_toggle(s)
            Event.WindowFullScreenToggle(
                isFullScreen = NativeWindowFullScreenToggleEvent.is_full_screen(nativeEvent),
            )
        }
        desktop_h.NativeEvent_WindowDraw() -> {
            val nativeEvent = NativeEvent.window_draw(s)
            Event.WindowDraw(
                softwareDrawData = SoftwareDrawData.fromNative(NativeWindowDrawEvent.software_draw_data(nativeEvent)),
                size = PhysicalSize.fromNative(NativeWindowDrawEvent.physical_size(nativeEvent)),
                scale = NativeWindowDrawEvent.scale(nativeEvent),
            )
        }
        desktop_h.NativeEvent_WindowScaleChanged() -> {
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
