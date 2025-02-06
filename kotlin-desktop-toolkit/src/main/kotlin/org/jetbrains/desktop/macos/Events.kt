package org.jetbrains.desktop.macos

import org.jetbrains.desktop.LogicalPixels
import org.jetbrains.desktop.LogicalPoint
import org.jetbrains.desktop.LogicalSize
import org.jetbrains.desktop.macos.generated.*
import org.jetbrains.desktop.macos.generated.Event as NativeEvent
import java.lang.foreign.MemorySegment

sealed class Event {
    companion object {}

    data class KeyDown(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val characters: String,
        val key: String,
        val modifiers: KeyModifiers,
        val isRepeat: Boolean
    ): Event()

    data class KeyUp(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val characters: String,
        val key: String,
        val modifiers: KeyModifiers,
    ): Event()

    data class ModifiersChanged(
        val windowId: WindowId,
        val modifiers: KeyModifiers,
        val keyCode: KeyCode
    ): Event()

    data class MouseMoved(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val locationInScreen: LogicalPoint,
        val pressedButtons: MouseButtonsSet
    ): Event()

    data class MouseDragged(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val locationInScreen: LogicalPoint,
        val pressedButtons: MouseButtonsSet
    ): Event()

    data class MouseEntered(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val locationInScreen: LogicalPoint,
        val pressedButtons: MouseButtonsSet
    ): Event()

    data class MouseExited(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val locationInScreen: LogicalPoint,
        val pressedButtons: MouseButtonsSet
    ): Event()

    data class MouseUp(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val locationInScreen: LogicalPoint,
        val pressedButtons: MouseButtonsSet
    ): Event()

    data class MouseDown(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val locationInScreen: LogicalPoint,
        val pressedButtons: MouseButtonsSet
    ): Event()

    data class ScrollWheel(
        val windowId: WindowId,
        val scrollingDeltaX: LogicalPixels,
        val scrollingDeltaY: LogicalPixels,
        val hasPreciseScrillingDeltas: Boolean,
        val locationInWindow: LogicalPoint,
        val locationInScreen: LogicalPoint,
        val pressedButtons: MouseButtonsSet
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
            is KeyDown -> windowId
            is KeyUp -> windowId
            is MouseMoved -> windowId
            is MouseDragged -> windowId
            is MouseEntered -> windowId
            is MouseExited -> windowId
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
        desktop_macos_h.KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            Event.KeyDown(
                windowId = KeyDownEvent.window_id(nativeEvent),
                keyCode = KeyCode.fromNative(KeyDownEvent.code(nativeEvent)),
                characters = KeyDownEvent.characters(nativeEvent).getUtf8String(0),
                key = KeyDownEvent.key(nativeEvent).getUtf8String(0),
                modifiers = KeyModifiers.fromNative(KeyDownEvent.modifiers(nativeEvent)),
                isRepeat = KeyDownEvent.is_repeat(nativeEvent)
            )
        }
        desktop_macos_h.KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                windowId = KeyUpEvent.window_id(nativeEvent),
                characters = KeyUpEvent.characters(nativeEvent).getUtf8String(0),
                key = KeyUpEvent.key(nativeEvent).getUtf8String(0),
                modifiers = KeyModifiers.fromNative(KeyUpEvent.modifiers(nativeEvent)),
                keyCode = KeyCode.fromNative(KeyUpEvent.code(nativeEvent))
            )
        }
        desktop_macos_h.ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            Event.ModifiersChanged(
                windowId = ModifiersChangedEvent.window_id(nativeEvent),
                modifiers = KeyModifiers.fromNative(ModifiersChangedEvent.modifiers(nativeEvent)),
                keyCode = KeyCode.fromNative(ModifiersChangedEvent.code(nativeEvent))
            )
        }
        desktop_macos_h.MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = MouseMovedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(MouseMovedEvent.location_in_window(nativeEvent)),
                locationInScreen = LogicalPoint.fromNative(MouseMovedEvent.location_in_screen(nativeEvent)),
                pressedButtons = MouseButtonsSet(MouseMovedEvent.pressed_buttons(nativeEvent))
            )
        }
        desktop_macos_h.MouseDragged() -> {
            val nativeEvent = NativeEvent.mouse_dragged(s)
            Event.MouseDragged(
                windowId = MouseDraggedEvent.window_id(nativeEvent),
                button = MouseButton(MouseDraggedEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(MouseDraggedEvent.location_in_window(nativeEvent)),
                locationInScreen = LogicalPoint.fromNative(MouseDraggedEvent.location_in_screen(nativeEvent)),
                pressedButtons = MouseButtonsSet(MouseDraggedEvent.pressed_buttons(nativeEvent))
            )
        }
        desktop_macos_h.MouseEntered() -> {
            val nativeEvent = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                windowId = MouseEnteredEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(MouseEnteredEvent.location_in_window(nativeEvent)),
                locationInScreen = LogicalPoint.fromNative(MouseEnteredEvent.location_in_screen(nativeEvent)),
                pressedButtons = MouseButtonsSet(MouseEnteredEvent.pressed_buttons(nativeEvent))
            )
        }
        desktop_macos_h.MouseExited() -> {
            val nativeEvent = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                windowId = MouseExitedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(MouseExitedEvent.location_in_window(nativeEvent)),
                locationInScreen = LogicalPoint.fromNative(MouseExitedEvent.location_in_screen(nativeEvent)),
                pressedButtons = MouseButtonsSet(MouseExitedEvent.pressed_buttons(nativeEvent))
            )
        }
        desktop_macos_h.MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                windowId = MouseUpEvent.window_id(nativeEvent),
                button = MouseButton(MouseUpEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(MouseUpEvent.location_in_window(nativeEvent)),
                locationInScreen = LogicalPoint.fromNative(MouseUpEvent.location_in_screen(nativeEvent)),
                pressedButtons = MouseButtonsSet(MouseUpEvent.pressed_buttons(nativeEvent))
            )
        }
        desktop_macos_h.MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                windowId = MouseDownEvent.window_id(nativeEvent),
                button = MouseButton(MouseDownEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(MouseDownEvent.location_in_window(nativeEvent)),
                locationInScreen = LogicalPoint.fromNative(MouseDownEvent.location_in_screen(nativeEvent)),
                pressedButtons = MouseButtonsSet(MouseDownEvent.pressed_buttons(nativeEvent))
            )
        }
        desktop_macos_h.ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                windowId = ScrollWheelEvent.window_id(nativeEvent),
                scrollingDeltaX = ScrollWheelEvent.scrolling_delta_x(nativeEvent),
                scrollingDeltaY = ScrollWheelEvent.scrolling_delta_y(nativeEvent),
                hasPreciseScrillingDeltas = ScrollWheelEvent.has_precise_scrolling_deltas(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(ScrollWheelEvent.location_in_window(nativeEvent)),
                locationInScreen = LogicalPoint.fromNative(ScrollWheelEvent.location_in_screen(nativeEvent)),
                pressedButtons = MouseButtonsSet(ScrollWheelEvent.pressed_buttons(nativeEvent))
            )
        }
        desktop_macos_h.WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                windowId = WindowScreenChangeEvent.window_id(nativeEvent),
                newScreenId = WindowScreenChangeEvent.new_screen_id(nativeEvent)
            )
        }
        desktop_macos_h.WindowResize() -> {
            val nativeEvent = NativeEvent.window_resize(s)
            Event.WindowResize(
                windowId = WindowResizeEvent.window_id(nativeEvent),
                size = LogicalSize.fromNative(WindowResizeEvent.size(nativeEvent))
            )
        }
        desktop_macos_h.WindowMove() -> {
            val nativeEvent = NativeEvent.window_move(s)
            Event.WindowMove(
                windowId = WindowMoveEvent.window_id(nativeEvent),
                origin = LogicalPoint.fromNative(WindowMoveEvent.origin(nativeEvent))
            )
        }
        desktop_macos_h.WindowFocusChange() -> {
            val nativeEvent = NativeEvent.window_focus_change(s)
            Event.WindowFocusChange(
                windowId = WindowFocusChangeEvent.window_id(nativeEvent),
                isKeyWindow = WindowFocusChangeEvent.is_key(nativeEvent),
                isMainWindow = WindowFocusChangeEvent.is_main(nativeEvent)
            )
        }
        desktop_macos_h.WindowCloseRequest() -> {
            val nativeEvent = NativeEvent.window_close_request(s)
            Event.WindowCloseRequest(
                windowId = WindowCloseRequestEvent.window_id(nativeEvent)
            )
        }
        desktop_macos_h.DisplayConfigurationChange() -> {
            Event.DisplayConfigurationChange
        }
        desktop_macos_h.ApplicationDidFinishLaunching() -> {
            Event.ApplicationDidFinishLaunching
        }
        desktop_macos_h.WindowFullScreenToggle() -> {
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