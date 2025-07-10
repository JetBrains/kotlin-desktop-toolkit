package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeEvent
import org.jetbrains.desktop.win32.generated.NativeWindowDrawEvent
import org.jetbrains.desktop.win32.generated.NativeWindowScaleChangedEvent
import org.jetbrains.desktop.win32.generated.desktop_windows_h
import java.lang.foreign.MemorySegment
import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds

@JvmInline
public value class Timestamp(
    /** Count of milliseconds since some fixed but arbitrary moment in the past */
    private val value: Int,
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

    public data object WindowCloseRequest : Event()

    public data class WindowDraw(
        val size: PhysicalSize,
        val scale: Float,
    ) : Event()

    public data class WindowScaleChanged(
        val newOrigin: PhysicalPoint,
        val newSize: PhysicalSize,
        val newScale: Float,
    ) : Event()
}

internal fun Event.Companion.fromNative(s: MemorySegment): Event {
    return when (NativeEvent.tag(s)) {
        desktop_windows_h.NativeEvent_WindowCloseRequest() -> {
            Event.WindowCloseRequest
        }

        desktop_windows_h.NativeEvent_WindowDraw() -> {
            val nativeEvent = NativeEvent.window_draw(s)
            Event.WindowDraw(
                size = PhysicalSize.fromNative(NativeWindowDrawEvent.physical_size(nativeEvent)),
                scale = NativeWindowDrawEvent.scale(nativeEvent),
            )
        }

        desktop_windows_h.NativeEvent_WindowScaleChanged() -> {
            val nativeEvent = NativeEvent.window_scale_changed(s)
            Event.WindowScaleChanged(
                newOrigin = PhysicalPoint.fromNative(NativeWindowScaleChangedEvent.new_origin(nativeEvent)),
                newSize = PhysicalSize.fromNative(NativeWindowScaleChangedEvent.new_size(nativeEvent)),
                newScale = NativeWindowScaleChangedEvent.new_scale(nativeEvent)
            )
        }

        else -> error("Unexpected Event tag")
    }
}
