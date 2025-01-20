package org.jetbrains.kwm.macos

import org.jetbrains.kwm.LogicalPixels
import org.jetbrains.kwm.LogicalPoint
import org.jetbrains.kwm.LogicalSize
import org.jetbrains.kwm.macos.generated.*
import org.jetbrains.kwm.macos.generated.Event as NativeEvent
import java.lang.foreign.MemorySegment

sealed class Event {
    companion object {}

    data class KeyDown(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val isRepeat: Boolean
    ): Event()

    data class KeyUp(
        val windowId: WindowId,
        val keyCode: KeyCode
    ): Event()

    data class MouseMoved(
        val windowId: WindowId,
        val point: LogicalPoint
    ): Event()

    data class MouseUp(
        val windowId: WindowId,
        val point: LogicalPoint
    ): Event()

    data class MouseDown(
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

    data class WindowMove(
        val windowId: WindowId,
        // bottom left corner of window
        val origin: LogicalPoint
    ): Event()

    data class WindowFocusChange(
        val windowId: WindowId,
        val isKeyWindow: Boolean,
        val isMainWindow: Boolean
    ): Event()

    data class WindowFullScreenToggle(
        val windowId: WindowId,
        val isFullScreen: Boolean
    ): Event()

    data object DisplayConfigurationChange: Event()

    data object ApplicationDidFinishLaunching: Event()

    data class WindowCloseRequest(
        val windowId: WindowId,
    ): Event()

    fun windowId(): WindowId? {
        return when (this) {
            is MouseMoved -> windowId
            is MouseUp -> windowId
            is MouseDown -> windowId
            is ScrollWheel -> windowId
            is WindowScreenChange -> windowId
            is WindowResize -> windowId
            is WindowMove -> windowId
            is WindowFocusChange -> windowId
            is WindowCloseRequest -> windowId
            is WindowFullScreenToggle -> windowId
            else -> null
        }
    }
}

fun Event.Companion.fromNative(s: MemorySegment): Event {
    return when (NativeEvent.tag(s)) {
        kwm_macos_h.KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            Event.KeyDown(
                windowId = KeyDownEvent.window_id(nativeEvent),
                keyCode = KeyCode.fromNative(KeyDownEvent.code(nativeEvent)),
                isRepeat = KeyDownEvent.is_repeat(nativeEvent)
            )
        }
        kwm_macos_h.KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                windowId = KeyUpEvent.window_id(nativeEvent),
                keyCode = KeyCode.fromNative(KeyUpEvent.code(nativeEvent))
            )
        }
        kwm_macos_h.MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = MouseMovedEvent.window_id(nativeEvent),
                point = LogicalPoint.fromNative(MouseMovedEvent.point(nativeEvent))
            )
        }
        kwm_macos_h.MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                windowId = MouseUpEvent.window_id(nativeEvent),
                point = LogicalPoint.fromNative(MouseUpEvent.point(nativeEvent))
            )
        }
        kwm_macos_h.MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                windowId = MouseDownEvent.window_id(nativeEvent),
                point = LogicalPoint.fromNative(MouseDownEvent.point(nativeEvent))
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
        kwm_macos_h.WindowMove() -> {
            val nativeEvent = NativeEvent.window_move(s)
            Event.WindowMove(
                windowId = WindowMoveEvent.window_id(nativeEvent),
                origin = LogicalPoint.fromNative(WindowMoveEvent.origin(nativeEvent))
            )
        }
        kwm_macos_h.WindowFocusChange() -> {
            val nativeEvent = NativeEvent.window_focus_change(s)
            Event.WindowFocusChange(
                windowId = WindowFocusChangeEvent.window_id(nativeEvent),
                isKeyWindow = WindowFocusChangeEvent.is_key(nativeEvent),
                isMainWindow = WindowFocusChangeEvent.is_main(nativeEvent)
            )
        }
        kwm_macos_h.WindowCloseRequest() -> {
            val nativeEvent = NativeEvent.window_close_request(s)
            Event.WindowCloseRequest(
                windowId = WindowCloseRequestEvent.window_id(nativeEvent)
            )
        }
        kwm_macos_h.DisplayConfigurationChange() -> {
            Event.DisplayConfigurationChange
        }
        kwm_macos_h.ApplicationDidFinishLaunching() -> {
            Event.ApplicationDidFinishLaunching
        }
        kwm_macos_h.WindowFullScreenToggle() -> {
            val nativeEvent = NativeEvent.window_full_screen_toggle(s)
            Event.WindowFullScreenToggle(
                windowId = WindowFullScreenToggleEvent.window_id(nativeEvent),
                isFullScreen = WindowFullScreenToggleEvent.is_full_screen(nativeEvent)
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}