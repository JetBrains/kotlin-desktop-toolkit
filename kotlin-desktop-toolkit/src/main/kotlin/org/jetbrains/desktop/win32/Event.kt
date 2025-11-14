package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeCharacterReceivedEvent
import org.jetbrains.desktop.win32.generated.NativeEvent
import org.jetbrains.desktop.win32.generated.NativeKeyEvent
import org.jetbrains.desktop.win32.generated.NativeNCCalcSizeEvent
import org.jetbrains.desktop.win32.generated.NativeNCHitTestEvent
import org.jetbrains.desktop.win32.generated.NativePointerButtonEvent
import org.jetbrains.desktop.win32.generated.NativePointerEnteredEvent
import org.jetbrains.desktop.win32.generated.NativePointerExitedEvent
import org.jetbrains.desktop.win32.generated.NativePointerUpdatedEvent
import org.jetbrains.desktop.win32.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.win32.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.win32.generated.NativeWindowMoveEvent
import org.jetbrains.desktop.win32.generated.NativeWindowResizeEvent
import org.jetbrains.desktop.win32.generated.NativeWindowResizeKind
import org.jetbrains.desktop.win32.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.win32.generated.NativeWindowTitleChangedEvent
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

    public data class CharacterReceived(
        val keyCode: Char,
        val characters: String,
        val keyStatus: PhysicalKeyStatus,
        val isDeadChar: Boolean,
        val isSystemKey: Boolean,
    ) : Event()

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

    public data class NCCalcSize(
        val origin: PhysicalPoint,
        val size: PhysicalSize,
        val scale: Float,
    ) : Event()

    public data class NCHitTest(
        val mouseX: Int,
        val mouseY: Int,
    ) : Event()

    public data class PointerDown(
        val button: PointerButtons,
        val locationInWindow: LogicalPoint,
        val state: PointerState,
        val timestamp: Timestamp,
    ) : Event()

    public data class PointerEntered(
        val locationInWindow: LogicalPoint,
        val state: PointerState,
        val timestamp: Timestamp,
    ) : Event()

    public data class PointerExited(val timestamp: Timestamp) : Event()

    public data class PointerUpdated(
        val locationInWindow: LogicalPoint,
        val state: PointerState,
        val timestamp: Timestamp,
    ) : Event()

    public data class PointerUp(
        val button: PointerButtons,
        val locationInWindow: LogicalPoint,
        val state: PointerState,
        val timestamp: Timestamp,
    ) : Event()

    public data class ScrollWheelX(
        val scrollingDelta: Int,
        val locationInWindow: LogicalPoint,
        val state: PointerState,
        val timestamp: Timestamp,
    ) : Event()

    public data class ScrollWheelY(
        val scrollingDelta: Int,
        val locationInWindow: LogicalPoint,
        val state: PointerState,
        val timestamp: Timestamp,
    ) : Event()

    public data object WindowCloseRequest : Event()

    public data class WindowDraw(
        val size: PhysicalSize,
        val scale: Float,
    ) : Event()

    public data object WindowKeyboardEnter : Event()

    public data object WindowKeyboardLeave : Event()

    public data class WindowMove(
        val origin: PhysicalPoint,
        val scale: Float,
    ) : Event()

    public data class WindowResize(
        val size: PhysicalSize,
        val scale: Float,
        val kind: WindowResizeKind,
    ) : Event()

    public data class WindowScaleChanged(
        val origin: PhysicalPoint,
        val size: PhysicalSize,
        val scale: Float,
    ) : Event()

    public data class WindowTitleChanged(val title: String) : Event()
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
    desktop_win32_h.NativeEvent_CharacterReceived() -> characterReceived(s)
    desktop_win32_h.NativeEvent_KeyDown() -> keyDown(s)
    desktop_win32_h.NativeEvent_KeyUp() -> keyUp(s)
    desktop_win32_h.NativeEvent_NCCalcSize() -> ncCalcSize(s)
    desktop_win32_h.NativeEvent_NCHitTest() -> ncHitTest(s)
    desktop_win32_h.NativeEvent_PointerDown() -> pointerDown(s)
    desktop_win32_h.NativeEvent_PointerEntered() -> pointerEntered(s)
    desktop_win32_h.NativeEvent_PointerExited() -> pointerExited(s)
    desktop_win32_h.NativeEvent_PointerUpdated() -> pointerUpdated(s)
    desktop_win32_h.NativeEvent_PointerUp() -> pointerUp(s)
    desktop_win32_h.NativeEvent_ScrollWheelX() -> scrollWheelX(s)
    desktop_win32_h.NativeEvent_ScrollWheelY() -> scrollWheelY(s)
    desktop_win32_h.NativeEvent_WindowCloseRequest() -> Event.WindowCloseRequest
    desktop_win32_h.NativeEvent_WindowDraw() -> windowDraw(s)
    desktop_win32_h.NativeEvent_WindowKeyboardEnter() -> Event.WindowKeyboardEnter
    desktop_win32_h.NativeEvent_WindowKeyboardLeave() -> Event.WindowKeyboardLeave
    desktop_win32_h.NativeEvent_WindowMove() -> windowMove(s)
    desktop_win32_h.NativeEvent_WindowResize() -> windowResize(s)
    desktop_win32_h.NativeEvent_WindowScaleChanged() -> windowScaleChanged(s)
    desktop_win32_h.NativeEvent_WindowTitleChanged() -> windowTitleChanged(s)
    else -> error("Unexpected Event tag")
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

private fun ncCalcSize(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.nc_calc_size(s)
    return Event.NCCalcSize(
        origin = PhysicalPoint.fromNative(NativeNCCalcSizeEvent.origin(nativeEvent)),
        size = PhysicalSize.fromNative(NativeNCCalcSizeEvent.size(nativeEvent)),
        scale = NativeNCCalcSizeEvent.scale(nativeEvent),
    )
}

private fun ncHitTest(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.nc_hit_test(s)
    return Event.NCHitTest(
        mouseX = NativeNCHitTestEvent.mouse_x(nativeEvent),
        mouseY = NativeNCHitTestEvent.mouse_y(nativeEvent),
    )
}

private fun pointerDown(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.pointer_down(s)
    return Event.PointerDown(
        button = PointerButtons(NativePointerButtonEvent.button(nativeEvent)),
        state = PointerState.fromNative(NativePointerButtonEvent.state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativePointerButtonEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativePointerButtonEvent.timestamp(nativeEvent)),
    )
}

private fun pointerEntered(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.pointer_entered(s)
    return Event.PointerEntered(
        state = PointerState.fromNative(NativePointerEnteredEvent.state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativePointerEnteredEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativePointerEnteredEvent.timestamp(nativeEvent)),
    )
}

private fun pointerExited(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.pointer_exited(s)
    return Event.PointerExited(
        timestamp = Timestamp(NativePointerExitedEvent.timestamp(nativeEvent)),
    )
}

private fun pointerUpdated(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.pointer_updated(s)
    return Event.PointerUpdated(
        state = PointerState.fromNative(NativePointerUpdatedEvent.state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativePointerUpdatedEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativePointerUpdatedEvent.timestamp(nativeEvent)),
    )
}

private fun pointerUp(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.pointer_up(s)
    return Event.PointerUp(
        button = PointerButtons(NativePointerButtonEvent.button(nativeEvent)),
        state = PointerState.fromNative(NativePointerButtonEvent.state(nativeEvent)),
        locationInWindow = LogicalPoint.fromNative(NativePointerButtonEvent.location_in_window(nativeEvent)),
        timestamp = Timestamp(NativePointerButtonEvent.timestamp(nativeEvent)),
    )
}

private fun scrollWheelX(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.scroll_wheel_x(s)
    return Event.ScrollWheelX(
        scrollingDelta = NativeScrollWheelEvent.scrolling_delta(nativeEvent),
        locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
        state = PointerState.fromNative(NativeScrollWheelEvent.state(nativeEvent)),
        timestamp = Timestamp(NativeScrollWheelEvent.timestamp(nativeEvent)),
    )
}

private fun scrollWheelY(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.scroll_wheel_y(s)
    return Event.ScrollWheelY(
        scrollingDelta = NativeScrollWheelEvent.scrolling_delta(nativeEvent),
        locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
        state = PointerState.fromNative(NativeScrollWheelEvent.state(nativeEvent)),
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

private fun windowMove(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.window_move(s)
    return Event.WindowMove(
        origin = PhysicalPoint.fromNative(NativeWindowMoveEvent.origin(nativeEvent)),
        scale = NativeWindowMoveEvent.scale(nativeEvent),
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

private fun windowScaleChanged(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.window_scale_changed(s)
    return Event.WindowScaleChanged(
        origin = PhysicalPoint.fromNative(NativeWindowScaleChangedEvent.origin(nativeEvent)),
        size = PhysicalSize.fromNative(NativeWindowScaleChangedEvent.size(nativeEvent)),
        scale = NativeWindowScaleChangedEvent.scale(nativeEvent),
    )
}

private fun windowTitleChanged(s: MemorySegment): Event {
    val nativeEvent = NativeEvent.window_title_changed(s)
    return Event.WindowTitleChanged(
        title = NativeWindowTitleChangedEvent.title(nativeEvent).getUtf8String(0),
    )
}
