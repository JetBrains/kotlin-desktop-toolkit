package org.jetbrains.desktop.macos

import org.jetbrains.desktop.LogicalPixels
import org.jetbrains.desktop.LogicalPoint
import org.jetbrains.desktop.LogicalSize
import org.jetbrains.desktop.macos.generated.KeyDownEvent
import org.jetbrains.desktop.macos.generated.KeyUpEvent
import org.jetbrains.desktop.macos.generated.ModifiersChangedEvent
import org.jetbrains.desktop.macos.generated.MouseDownEvent
import org.jetbrains.desktop.macos.generated.MouseDraggedEvent
import org.jetbrains.desktop.macos.generated.MouseEnteredEvent
import org.jetbrains.desktop.macos.generated.MouseExitedEvent
import org.jetbrains.desktop.macos.generated.MouseMovedEvent
import org.jetbrains.desktop.macos.generated.MouseUpEvent
import org.jetbrains.desktop.macos.generated.ScrollWheelEvent
import org.jetbrains.desktop.macos.generated.WindowCloseRequestEvent
import org.jetbrains.desktop.macos.generated.WindowFocusChangeEvent
import org.jetbrains.desktop.macos.generated.WindowFullScreenToggleEvent
import org.jetbrains.desktop.macos.generated.WindowMoveEvent
import org.jetbrains.desktop.macos.generated.WindowResizeEvent
import org.jetbrains.desktop.macos.generated.WindowScreenChangeEvent
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds
import org.jetbrains.desktop.macos.generated.Event as NativeEvent

@JvmInline
public value class Timestamp(
    private val value: Double, // Count of seconds passed since some fixed but arbitrary moment in the past
) {
    public fun toDuration(): Duration {
        return value.seconds
    }
}

public sealed class Event {
    public companion object {
        public fun pressedMouseButtons(): MouseButtonsSet {
            return MouseButtonsSet(desktop_macos_h.events_pressed_mouse_buttons())
        }

        public fun pressedModifiers(): KeyModifiersSet {
            return KeyModifiersSet(desktop_macos_h.events_pressed_modifiers())
        }

        public fun cursorLocationInScreen(): LogicalPoint {
            return Arena.ofConfined().use { arena ->
                LogicalPoint.fromNative(desktop_macos_h.events_cursor_location_in_screen(arena))
            }
        }
    }

    public data class KeyDown(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val characters: String,
        val key: String,
        val modifiers: KeyModifiersSet,
        val isRepeat: Boolean,
        val timestamp: Timestamp,
    ) : Event()

    public data class KeyUp(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val characters: String,
        val key: String,
        val modifiers: KeyModifiersSet,
        val timestamp: Timestamp,
    ) : Event()

    public data class ModifiersChanged(
        val windowId: WindowId,
        val modifiers: KeyModifiersSet,
        val keyCode: KeyCode,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseMoved(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseDragged(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseEntered(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseExited(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseUp(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseDown(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class ScrollWheel(
        val windowId: WindowId,
        val scrollingDeltaX: LogicalPixels,
        val scrollingDeltaY: LogicalPixels,
        val hasPreciseScrillingDeltas: Boolean,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class WindowScreenChange(
        val windowId: WindowId,
        val newScreenId: ScreenId,
    ) : Event()

    public data class WindowResize(
        val windowId: WindowId,
        val size: LogicalSize,
    ) : Event()

    public data class WindowMove(
        val windowId: WindowId,
        val origin: LogicalPoint,
    ) : Event()

    public data class WindowFocusChange(
        val windowId: WindowId,
        val isKeyWindow: Boolean,
        val isMainWindow: Boolean,
    ) : Event()

    public data class WindowFullScreenToggle(
        val windowId: WindowId,
        val isFullScreen: Boolean,
    ) : Event()

    public data object DisplayConfigurationChange : Event()

    public data object ApplicationDidFinishLaunching : Event()

    public data class WindowCloseRequest(val windowId: WindowId) : Event()

    public fun windowId(): WindowId? {
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

internal fun Event.Companion.fromNative(s: MemorySegment): Event {
    return when (NativeEvent.tag(s)) {
        desktop_macos_h.Event_KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            Event.KeyDown(
                windowId = KeyDownEvent.window_id(nativeEvent),
                keyCode = KeyCode.fromNative(KeyDownEvent.code(nativeEvent)),
                characters = KeyDownEvent.characters(nativeEvent).getUtf8String(0),
                key = KeyDownEvent.key(nativeEvent).getUtf8String(0),
                modifiers = KeyModifiersSet(KeyDownEvent.modifiers(nativeEvent)),
                isRepeat = KeyDownEvent.is_repeat(nativeEvent),
                timestamp = Timestamp(KeyDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                windowId = KeyUpEvent.window_id(nativeEvent),
                characters = KeyUpEvent.characters(nativeEvent).getUtf8String(0),
                key = KeyUpEvent.key(nativeEvent).getUtf8String(0),
                modifiers = KeyModifiersSet(KeyUpEvent.modifiers(nativeEvent)),
                keyCode = KeyCode.fromNative(KeyUpEvent.code(nativeEvent)),
                timestamp = Timestamp(KeyUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            Event.ModifiersChanged(
                windowId = ModifiersChangedEvent.window_id(nativeEvent),
                modifiers = KeyModifiersSet(ModifiersChangedEvent.modifiers(nativeEvent)),
                keyCode = KeyCode.fromNative(ModifiersChangedEvent.code(nativeEvent)),
                timestamp = Timestamp(ModifiersChangedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = MouseMovedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(MouseMovedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(MouseMovedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_MouseDragged() -> {
            val nativeEvent = NativeEvent.mouse_dragged(s)
            Event.MouseDragged(
                windowId = MouseDraggedEvent.window_id(nativeEvent),
                button = MouseButton(MouseDraggedEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(MouseDraggedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(MouseDraggedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_MouseEntered() -> {
            val nativeEvent = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                windowId = MouseEnteredEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(MouseEnteredEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(MouseEnteredEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_MouseExited() -> {
            val nativeEvent = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                windowId = MouseExitedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(MouseExitedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(MouseExitedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                windowId = MouseUpEvent.window_id(nativeEvent),
                button = MouseButton(MouseUpEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(MouseUpEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(MouseUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                windowId = MouseDownEvent.window_id(nativeEvent),
                button = MouseButton(MouseDownEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(MouseDownEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(MouseDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.Event_ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                windowId = ScrollWheelEvent.window_id(nativeEvent),
                scrollingDeltaX = ScrollWheelEvent.scrolling_delta_x(nativeEvent),
                scrollingDeltaY = ScrollWheelEvent.scrolling_delta_y(nativeEvent),
                hasPreciseScrillingDeltas = ScrollWheelEvent.has_precise_scrolling_deltas(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(ScrollWheelEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(ScrollWheelEvent.timestamp(nativeEvent)),
            )
        } desktop_macos_h.Event_WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                windowId = WindowScreenChangeEvent.window_id(nativeEvent),
                newScreenId = WindowScreenChangeEvent.new_screen_id(nativeEvent),
            )
        }
        desktop_macos_h.Event_WindowResize() -> {
            val nativeEvent = NativeEvent.window_resize(s)
            Event.WindowResize(
                windowId = WindowResizeEvent.window_id(nativeEvent),
                size = LogicalSize.fromNative(WindowResizeEvent.size(nativeEvent)),
            )
        }
        desktop_macos_h.Event_WindowMove() -> {
            val nativeEvent = NativeEvent.window_move(s)
            Event.WindowMove(
                windowId = WindowMoveEvent.window_id(nativeEvent),
                origin = LogicalPoint.fromNative(WindowMoveEvent.origin(nativeEvent)),
            )
        }
        desktop_macos_h.Event_WindowFocusChange() -> {
            val nativeEvent = NativeEvent.window_focus_change(s)
            Event.WindowFocusChange(
                windowId = WindowFocusChangeEvent.window_id(nativeEvent),
                isKeyWindow = WindowFocusChangeEvent.is_key(nativeEvent),
                isMainWindow = WindowFocusChangeEvent.is_main(nativeEvent),
            )
        }
        desktop_macos_h.Event_WindowCloseRequest() -> {
            val nativeEvent = NativeEvent.window_close_request(s)
            Event.WindowCloseRequest(
                windowId = WindowCloseRequestEvent.window_id(nativeEvent),
            )
        }
        desktop_macos_h.Event_DisplayConfigurationChange() -> {
            Event.DisplayConfigurationChange
        }
        desktop_macos_h.Event_ApplicationDidFinishLaunching() -> {
            Event.ApplicationDidFinishLaunching
        }
        desktop_macos_h.Event_WindowFullScreenToggle() -> {
            val nativeEvent = NativeEvent.window_full_screen_toggle(s)
            Event.WindowFullScreenToggle(
                windowId = WindowFullScreenToggleEvent.window_id(nativeEvent),
                isFullScreen = WindowFullScreenToggleEvent.is_full_screen(nativeEvent),
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}
