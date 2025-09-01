package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeCharacterReceivedEvent
import org.jetbrains.desktop.win32.generated.NativeEvent
import org.jetbrains.desktop.win32.generated.NativeKeyEvent
import org.jetbrains.desktop.win32.generated.NativeMouseButtonEvent
import org.jetbrains.desktop.win32.generated.NativeMouseEnteredEvent
import org.jetbrains.desktop.win32.generated.NativeMouseExitedEvent
import org.jetbrains.desktop.win32.generated.NativeMouseMovedEvent
import org.jetbrains.desktop.win32.generated.NativeNCHitTestEvent
import org.jetbrains.desktop.win32.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.win32.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.win32.generated.NativeWindowResizeEvent
import org.jetbrains.desktop.win32.generated.NativeWindowResizeKind
import org.jetbrains.desktop.win32.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.MemorySegment
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds

@JvmInline
public value class Timestamp(
    /** Count of milliseconds since some fixed but arbitrary moment in the past */
    private val value: Long,
) {
    public fun toDuration(): Duration {
        return value.milliseconds
    }
}

public enum class EventHandlerResult {
    Continue,
    Stop,
}

public typealias EventHandler = (WindowId, Event) -> EventHandlerResult

public sealed class Event {
    internal companion object;

    public data class KeyDown(
        val keyCode: VirtualKey,
        val keyStatus: PhysicalKeyStatus,
        val isSystemKey: Boolean,
        val timestamp: Timestamp,
    ) : Event()

    public data class KeyUp(
        val keyCode: VirtualKey,
        val keyStatus: PhysicalKeyStatus,
        val isSystemKey: Boolean,
        val timestamp: Timestamp,
    ) : Event()

    public data class CharacterReceived(
        val keyCode: Char,
        val characters: String,
        val keyStatus: PhysicalKeyStatus,
        val isDeadChar: Boolean,
        val isSystemKey: Boolean,
    ) : Event()

    public data class MouseEntered(
        val keyState: MouseKeyState,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseExited(val timestamp: Timestamp) : Event()

    public data class MouseMoved(
        val keyState: MouseKeyState,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseDown(
        val button: MouseButton,
        val keyState: MouseKeyState,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseUp(
        val button: MouseButton,
        val keyState: MouseKeyState,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class NCHitTest(
        val mouseX: Int,
        val mouseY: Int,
    ) : Event()

    public data class ScrollWheelX(
        val scrollingDelta: Short,
        val keyState: MouseKeyState,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class ScrollWheelY(
        val scrollingDelta: Short,
        val keyState: MouseKeyState,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data object WindowCloseRequest : Event()

    public data class WindowDraw(
        val size: PhysicalSize,
        val scale: Float,
    ) : Event()

    public data object WindowKeyboardEnter : Event()

    public data object WindowKeyboardLeave : Event()

    public data class WindowScaleChanged(
        val newOrigin: PhysicalPoint,
        val newSize: PhysicalSize,
        val newScale: Float,
    ) : Event()

    public data class WindowResize(
        val size: PhysicalSize,
        val scale: Float,
        val kind: WindowResizeKind,
    ) : Event()
}

public sealed class WindowResizeKind {
    internal companion object {
        internal fun fromNative(s: MemorySegment): WindowResizeKind = when (NativeWindowResizeKind.tag(s)) {
            desktop_win32_h.NativeWindowResizeKind_Restored() -> Restored
            desktop_win32_h.NativeWindowResizeKind_Maximized() -> Maximized
            desktop_win32_h.NativeWindowResizeKind_Minimized() -> Minimized
            desktop_win32_h.NativeWindowResizeKind_Other() -> Other(NativeWindowResizeKind.other(s).toUInt())
            else -> error("Unexpected WindowResizeKind tag")
        }
    }

    public data object Restored : WindowResizeKind()

    public data object Maximized : WindowResizeKind()

    public data object Minimized : WindowResizeKind()

    public data class Other(val kind: UInt) : WindowResizeKind()
}

internal fun Event.Companion.fromNative(s: MemorySegment): Event = when (NativeEvent.tag(s)) {
    desktop_win32_h.NativeEvent_KeyDown() -> keyDown(s)
    desktop_win32_h.NativeEvent_KeyUp() -> keyUp(s)
    desktop_win32_h.NativeEvent_CharacterReceived() -> characterReceived(s)
    desktop_win32_h.NativeEvent_MouseEntered() -> mouseEntered(s)
    desktop_win32_h.NativeEvent_MouseExited() -> mouseExited(s)
    desktop_win32_h.NativeEvent_MouseMoved() -> mouseMoved(s)
    desktop_win32_h.NativeEvent_MouseDown() -> mouseDown(s)
    desktop_win32_h.NativeEvent_MouseUp() -> mouseUp(s)
    desktop_win32_h.NativeEvent_NCHitTest() -> ncHitTest(s)
    desktop_win32_h.NativeEvent_ScrollWheelX() -> scrollWheelX(s)
    desktop_win32_h.NativeEvent_ScrollWheelY() -> scrollWheelY(s)
    desktop_win32_h.NativeEvent_WindowCloseRequest() -> Event.WindowCloseRequest
    desktop_win32_h.NativeEvent_WindowDraw() -> windowDraw(s)
    desktop_win32_h.NativeEvent_WindowKeyboardEnter() -> Event.WindowKeyboardEnter
    desktop_win32_h.NativeEvent_WindowKeyboardLeave() -> Event.WindowKeyboardLeave
    desktop_win32_h.NativeEvent_WindowScaleChanged() -> windowScaleChanged(s)
    desktop_win32_h.NativeEvent_WindowResize() -> windowResize(s)
    else -> error("Unexpected Event tag")
}

private fun keyDown(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.key_down(s)
    return Event.KeyDown(
        keyCode = VirtualKey.fromNative(NativeKeyEvent.key_code(nativeEvent)),
        keyStatus = PhysicalKeyStatus.fromNative(NativeKeyEvent.key_status(nativeEvent)),
        isSystemKey = NativeKeyEvent.is_system_key(nativeEvent),
        timestamp = Timestamp(NativeKeyEvent.timestamp(nativeEvent)),
    )
}

private fun keyUp(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.key_up(s)
    return Event.KeyUp(
        keyCode = VirtualKey.fromNative(NativeKeyEvent.key_code(nativeEvent)),
        keyStatus = PhysicalKeyStatus.fromNative(NativeKeyEvent.key_status(nativeEvent)),
        isSystemKey = NativeKeyEvent.is_system_key(nativeEvent),
        timestamp = Timestamp(NativeKeyEvent.timestamp(nativeEvent)),
    )
}

private fun characterReceived(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.character_received(s)
    return Event.CharacterReceived(
        keyCode = NativeCharacterReceivedEvent.key_code(nativeEvent).toInt().toChar(),
        characters = NativeCharacterReceivedEvent.characters(nativeEvent).getUtf8String(0),
        keyStatus = PhysicalKeyStatus.fromNative(NativeCharacterReceivedEvent.key_status(nativeEvent)),
        isDeadChar = NativeCharacterReceivedEvent.is_dead_char(nativeEvent),
        isSystemKey = NativeCharacterReceivedEvent.is_system_key(nativeEvent),
    )
}

private fun mouseEntered(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.mouse_entered(s)
    return Event.MouseEntered(
        keyState = MouseKeyState.fromNative(NativeMouseEnteredEvent.key_state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativeMouseEnteredEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativeMouseEnteredEvent.timestamp(nativeEvent)),
    )
}

private fun mouseExited(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.mouse_exited(s)
    return Event.MouseExited(
        timestamp = Timestamp(NativeMouseExitedEvent.timestamp(nativeEvent)),
    )
}

private fun mouseMoved(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.mouse_moved(s)
    return Event.MouseMoved(
        keyState = MouseKeyState.fromNative(NativeMouseMovedEvent.key_state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativeMouseMovedEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativeMouseMovedEvent.timestamp(nativeEvent)),
    )
}

private fun mouseDown(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.mouse_down(s)
    return Event.MouseDown(
        button = MouseButton.fromNative(NativeMouseButtonEvent.button(nativeEvent)),
        keyState = MouseKeyState.fromNative(NativeMouseButtonEvent.key_state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativeMouseButtonEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativeMouseButtonEvent.timestamp(nativeEvent)),
    )
}

private fun mouseUp(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.mouse_up(s)
    return Event.MouseUp(
        button = MouseButton.fromNative(NativeMouseButtonEvent.button(nativeEvent)),
        keyState = MouseKeyState.fromNative(NativeMouseButtonEvent.key_state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativeMouseButtonEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativeMouseButtonEvent.timestamp(nativeEvent)),
    )
}

private fun ncHitTest(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.nc_hit_test(s)
    return Event.NCHitTest(
        mouseX = NativeNCHitTestEvent.mouse_x(nativeEvent),
        mouseY = NativeNCHitTestEvent.mouse_y(nativeEvent),
    )
}

private fun scrollWheelX(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.scroll_wheel_x(s)
    return Event.ScrollWheelX(
        scrollingDelta = NativeScrollWheelEvent.scrolling_delta(nativeEvent),
        keyState = MouseKeyState.fromNative(NativeScrollWheelEvent.key_state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativeScrollWheelEvent.timestamp(nativeEvent)),
    )
}

private fun scrollWheelY(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.scroll_wheel_y(s)
    return Event.ScrollWheelY(
        scrollingDelta = NativeScrollWheelEvent.scrolling_delta(nativeEvent),
        keyState = MouseKeyState.fromNative(NativeScrollWheelEvent.key_state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativeScrollWheelEvent.timestamp(nativeEvent)),
    )
}

private fun windowDraw(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.window_draw(s)
    return Event.WindowDraw(
        size = PhysicalSize.fromNative(NativeWindowDrawEvent.size(nativeEvent)),
        scale = NativeWindowDrawEvent.scale(nativeEvent),
    )
}

private fun windowScaleChanged(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.window_scale_changed(s)
    return Event.WindowScaleChanged(
        newOrigin = PhysicalPoint.fromNative(NativeWindowScaleChangedEvent.new_origin(nativeEvent)),
        newSize = PhysicalSize.fromNative(NativeWindowScaleChangedEvent.new_size(nativeEvent)),
        newScale = NativeWindowScaleChangedEvent.new_scale(nativeEvent),
    )
}

private fun windowResize(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.window_resize(s)
    return Event.WindowResize(
        size = PhysicalSize.fromNative(NativeWindowResizeEvent.size(nativeEvent)),
        scale = NativeWindowResizeEvent.scale(nativeEvent),
        kind = WindowResizeKind.fromNative(NativeWindowResizeEvent.kind(nativeEvent)),
    )
}
