package org.jetbrains.desktop.linux

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
import org.jetbrains.desktop.linux.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.linux.generated.NativeWindowFocusChangeEvent
import org.jetbrains.desktop.linux.generated.NativeWindowFullScreenToggleEvent
import org.jetbrains.desktop.linux.generated.NativeWindowMoveEvent
import org.jetbrains.desktop.linux.generated.NativeWindowResizeEvent
import org.jetbrains.desktop.linux.generated.NativeWindowScreenChangeEvent
import java.lang.foreign.MemorySegment
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds
import org.jetbrains.desktop.linux.generated.desktop_linux_h as desktop_h

@JvmInline
public value class Timestamp(
    // Count of seconds passed since some fixed but arbitrary moment in the past
    private val value: Int,
) {
    public fun toDuration(): Duration {
        return value.seconds
    }
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

    public data class KeyDown(
        val keyCode: KeyCode,
        val characters: String,
        val key: String,
        val modifiers: KeyModifiersSet,
        val isRepeat: Boolean,
        val timestamp: Timestamp,
    ) : Event()

    public data class KeyUp(
        val keyCode: KeyCode,
        val characters: String,
        val key: String,
        val modifiers: KeyModifiersSet,
        val timestamp: Timestamp,
    ) : Event()

    public data class ModifiersChanged(
        val modifiers: KeyModifiersSet,
        val keyCode: KeyCode,
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
    ) : Event()

    public data class WindowMove(
        val origin: LogicalPoint,
    ) : Event()

    public data class WindowFocusChange(
        val isKeyWindow: Boolean,
        val isMainWindow: Boolean,
    ) : Event()

    public data class WindowFullScreenToggle(
        val isFullScreen: Boolean,
    ) : Event()

    public data class WindowDraw(
        val buffer: Long,
        val width: Int,
        val height: Int,
        val stride: Int,
    ) : Event()

    public data object WindowCloseRequest : Event()
}

internal fun Event.Companion.fromNative(s: MemorySegment): Event {
    return when (NativeEvent.tag(s)) {
        desktop_h.NativeEvent_KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            Event.KeyDown(
                keyCode = KeyCode.fromNative(NativeKeyDownEvent.code(nativeEvent)),
                characters = NativeKeyDownEvent.characters(nativeEvent).getUtf8String(0),
                key = NativeKeyDownEvent.key(nativeEvent).getUtf8String(0),
                modifiers = KeyModifiersSet(NativeKeyDownEvent.modifiers(nativeEvent)),
                isRepeat = NativeKeyDownEvent.is_repeat(nativeEvent),
                timestamp = Timestamp(NativeKeyDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                characters = NativeKeyUpEvent.characters(nativeEvent).getUtf8String(0),
                key = NativeKeyUpEvent.key(nativeEvent).getUtf8String(0),
                modifiers = KeyModifiersSet(NativeKeyUpEvent.modifiers(nativeEvent)),
                keyCode = KeyCode.fromNative(NativeKeyUpEvent.code(nativeEvent)),
                timestamp = Timestamp(NativeKeyUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_h.NativeEvent_ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            Event.ModifiersChanged(
                modifiers = KeyModifiersSet(NativeModifiersChangedEvent.modifiers(nativeEvent)),
                keyCode = KeyCode.fromNative(NativeModifiersChangedEvent.code(nativeEvent)),
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
            )
        }
        desktop_h.NativeEvent_ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                scrollingDeltaX = NativeScrollWheelEvent.scrolling_delta_x(nativeEvent),
                scrollingDeltaY = NativeScrollWheelEvent.scrolling_delta_y(nativeEvent),
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
            )
        }
        desktop_h.NativeEvent_WindowMove() -> {
            val nativeEvent = NativeEvent.window_move(s)
            Event.WindowMove(
                origin = LogicalPoint.fromNative(NativeWindowMoveEvent.origin(nativeEvent)),
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
                buffer = NativeWindowDrawEvent.buffer(nativeEvent).address(),
                width = NativeWindowDrawEvent.width(nativeEvent),
                height = NativeWindowDrawEvent.height(nativeEvent),
                stride = NativeWindowDrawEvent.stride(nativeEvent),
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}
