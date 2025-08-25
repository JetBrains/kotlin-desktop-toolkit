package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeCharacterReceivedEvent
import org.jetbrains.desktop.win32.generated.NativeEvent
import org.jetbrains.desktop.win32.generated.NativeKeyEvent
import org.jetbrains.desktop.win32.generated.NativeNCHitTestEvent
import org.jetbrains.desktop.win32.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.win32.generated.NativeWindowResizeEvent
import org.jetbrains.desktop.win32.generated.NativeWindowResizeKind
import org.jetbrains.desktop.win32.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.win32.generated.desktop_windows_h
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

    public data class NCHitTest(
        val mouseX: Int,
        val mouseY: Int,
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
    internal companion object;

    public data object Restored : WindowResizeKind()

    public data object Maximized : WindowResizeKind()

    public data object Minimized : WindowResizeKind()

    public data class Other(val kind: UInt) : WindowResizeKind()
}

internal fun Event.Companion.fromNative(s: MemorySegment): Event = when (NativeEvent.tag(s)) {
    desktop_windows_h.NativeEvent_KeyDown() -> {
        val nativeEvent = NativeEvent.key_down(s)
        Event.KeyDown(
            keyCode = VirtualKey.fromNative(NativeKeyEvent.key_code(nativeEvent)),
            keyStatus = PhysicalKeyStatus.fromNative(NativeKeyEvent.key_status(nativeEvent)),
            isSystemKey = NativeKeyEvent.is_system_key(nativeEvent),
            timestamp = Timestamp(NativeKeyEvent.timestamp(nativeEvent)),
        )
    }

    desktop_windows_h.NativeEvent_KeyUp() -> {
        val nativeEvent = NativeEvent.key_up(s)
        Event.KeyUp(
            keyCode = VirtualKey.fromNative(NativeKeyEvent.key_code(nativeEvent)),
            keyStatus = PhysicalKeyStatus.fromNative(NativeKeyEvent.key_status(nativeEvent)),
            isSystemKey = NativeKeyEvent.is_system_key(nativeEvent),
            timestamp = Timestamp(NativeKeyEvent.timestamp(nativeEvent)),
        )
    }

    desktop_windows_h.NativeEvent_CharacterReceived() -> {
        val nativeEvent = NativeEvent.character_received(s)
        Event.CharacterReceived(
            keyCode = NativeCharacterReceivedEvent.key_code(nativeEvent).toInt().toChar(),
            characters = NativeCharacterReceivedEvent.characters(nativeEvent).getUtf8String(0),
            keyStatus = PhysicalKeyStatus.fromNative(NativeCharacterReceivedEvent.key_status(nativeEvent)),
            isDeadChar = NativeCharacterReceivedEvent.is_dead_char(nativeEvent),
            isSystemKey = NativeCharacterReceivedEvent.is_system_key(nativeEvent),
        )
    }

    desktop_windows_h.NativeEvent_NCHitTest() -> {
        val nativeEvent = NativeEvent.nc_hit_test(s)
        Event.NCHitTest(
            mouseX = NativeNCHitTestEvent.mouse_x(nativeEvent),
            mouseY = NativeNCHitTestEvent.mouse_y(nativeEvent),
        )
    }

    desktop_windows_h.NativeEvent_WindowCloseRequest() -> {
        Event.WindowCloseRequest
    }

    desktop_windows_h.NativeEvent_WindowDraw() -> {
        val nativeEvent = NativeEvent.window_draw(s)
        Event.WindowDraw(
            size = PhysicalSize.fromNative(NativeWindowDrawEvent.size(nativeEvent)),
            scale = NativeWindowDrawEvent.scale(nativeEvent),
        )
    }

    desktop_windows_h.NativeEvent_WindowKeyboardEnter() -> {
        Event.WindowKeyboardEnter
    }

    desktop_windows_h.NativeEvent_WindowKeyboardLeave() -> {
        Event.WindowKeyboardLeave
    }

    desktop_windows_h.NativeEvent_WindowScaleChanged() -> {
        val nativeEvent = NativeEvent.window_scale_changed(s)
        Event.WindowScaleChanged(
            newOrigin = PhysicalPoint.fromNative(NativeWindowScaleChangedEvent.new_origin(nativeEvent)),
            newSize = PhysicalSize.fromNative(NativeWindowScaleChangedEvent.new_size(nativeEvent)),
            newScale = NativeWindowScaleChangedEvent.new_scale(nativeEvent),
        )
    }

    desktop_windows_h.NativeEvent_WindowResize() -> {
        val nativeEvent = NativeEvent.window_resize(s)
        Event.WindowResize(
            size = PhysicalSize.fromNative(NativeWindowResizeEvent.size(nativeEvent)),
            scale = NativeWindowResizeEvent.scale(nativeEvent),
            kind = WindowResizeKind.fromNative(NativeWindowResizeEvent.kind(nativeEvent)),
        )
    }

    else -> error("Unexpected Event tag")
}

internal fun WindowResizeKind.Companion.fromNative(s: MemorySegment): WindowResizeKind = when (NativeWindowResizeKind.tag(s)) {
    desktop_windows_h.NativeWindowResizeKind_Restored() -> WindowResizeKind.Restored

    desktop_windows_h.NativeWindowResizeKind_Maximized() -> WindowResizeKind.Maximized

    desktop_windows_h.NativeWindowResizeKind_Minimized() -> WindowResizeKind.Minimized

    desktop_windows_h.NativeWindowResizeKind_Other() -> {
        WindowResizeKind.Other(NativeWindowResizeKind.other(s).toUInt())
    }

    else -> error("Unexpected WindowResizeKind tag")
}
