package org.jetbrains.kwm.macos

import org.jetbrains.kwm.LogicalPixels
import org.jetbrains.kwm.LogicalPoint
import org.jetbrains.kwm.LogicalSize
import org.jetbrains.kwm.macos.generated.MouseMovedEvent
import org.jetbrains.kwm.macos.generated.ScrollWheelEvent
import org.jetbrains.kwm.macos.generated.WindowResizeEvent
import org.jetbrains.kwm.macos.generated.WindowScreenChangeEvent
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import org.jetbrains.kwm.macos.generated.Event as NativeEvent
import java.lang.foreign.MemorySegment

sealed class Event {
    companion object {}

    data class MouseMoved(
        val windowId: WindowId,
        val point: LogicalPoint
    ): Event()

    data class ScrollWheel(
        val windowId: WindowId,
        val dx: LogicalPixels,
        val dy: LogicalPixels
    ): Event()

    data class WindowScreenChange(
        val windowId: WindowId,
        val newScreenId: ScreenId,
    ): Event()

    data class WindowResize(
        val windowId: WindowId,
        val size: LogicalSize
    ): Event()

    fun windowId(): WindowId? {
        return when (this) {
            is MouseMoved -> windowId
            is ScrollWheel -> windowId
            is WindowScreenChange -> windowId
            is WindowResize -> windowId
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
                point = LogicalPoint.fromNative(MouseMovedEvent.point(nativeEvent))
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
        kwm_macos_h.WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                windowId = WindowScreenChangeEvent.window_id(nativeEvent),
                newScreenId = WindowScreenChangeEvent.new_screen_id(nativeEvent)
            )
        }
        kwm_macos_h.WindowResize() -> {
            val nativeEvent = NativeEvent.window_resize(s)
            Event.WindowResize(
                windowId = WindowResizeEvent.window_id(nativeEvent),
                size = LogicalSize.fromNative(WindowResizeEvent.size(nativeEvent))
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}