package org.jetbrains.kwm.macos

import org.jetbrains.kwm.Point
import org.jetbrains.kwm.macos.generated.MouseMovedEvent
import org.jetbrains.kwm.macos.generated.ScrollWheelEvent
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import org.jetbrains.kwm.macos.generated.Event as NativeEvent
import java.lang.foreign.MemorySegment

sealed class Event {
    companion object {}

    data class MouseMoved(
        val windowId: WindowId,
        val point: Point
    ): Event()

    data class ScrollWheel(
        val windowId: WindowId,
        val dx: Double,
        val dy: Double
    ): Event()

    fun windowId(): WindowId? {
        return when (this) {
            is MouseMoved -> windowId
            is ScrollWheel -> windowId
            else -> null
        }
    }
}

fun Event.Companion.fromNative(s: MemorySegment): Event {
    return when (NativeEvent.tag(s)) {
        kwm_macos_h.MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = MouseMovedEvent.window_id(nativeEvent),
                point = Point.fromNative(MouseMovedEvent.point(nativeEvent))
            )
        }
        kwm_macos_h.ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                windowId = ScrollWheelEvent.window_id(nativeEvent),
                dx = ScrollWheelEvent.dx(nativeEvent),
                dy = ScrollWheelEvent.dy(nativeEvent)
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}