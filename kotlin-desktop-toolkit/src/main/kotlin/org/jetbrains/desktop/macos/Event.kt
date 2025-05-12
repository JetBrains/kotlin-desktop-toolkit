package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeApplicationAppearanceChangeEvent
import org.jetbrains.desktop.macos.generated.NativeEvent
import org.jetbrains.desktop.macos.generated.NativeKeyDownEvent
import org.jetbrains.desktop.macos.generated.NativeKeyUpEvent
import org.jetbrains.desktop.macos.generated.NativeModifiersChangedEvent
import org.jetbrains.desktop.macos.generated.NativeMouseDownEvent
import org.jetbrains.desktop.macos.generated.NativeMouseDraggedEvent
import org.jetbrains.desktop.macos.generated.NativeMouseEnteredEvent
import org.jetbrains.desktop.macos.generated.NativeMouseExitedEvent
import org.jetbrains.desktop.macos.generated.NativeMouseMovedEvent
import org.jetbrains.desktop.macos.generated.NativeMouseUpEvent
import org.jetbrains.desktop.macos.generated.NativeScrollWheelEvent
import org.jetbrains.desktop.macos.generated.NativeWindowCloseRequestEvent
import org.jetbrains.desktop.macos.generated.NativeWindowFocusChangeEvent
import org.jetbrains.desktop.macos.generated.NativeWindowFullScreenToggleEvent
import org.jetbrains.desktop.macos.generated.NativeWindowMoveEvent
import org.jetbrains.desktop.macos.generated.NativeWindowResizeEvent
import org.jetbrains.desktop.macos.generated.NativeWindowScreenChangeEvent
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

@JvmInline
public value class Timestamp(
    // Count of seconds passed since some fixed but arbitrary moment in the past
    private val value: Double,
) {
    public fun toDuration(): Duration {
        return value.seconds
    }
}

public interface WindowEvent {
    public val windowId: WindowId
}

public sealed class Event {
    public companion object {
        public fun pressedMouseButtons(): MouseButtonsSet {
            return MouseButtonsSet(ffiDownCall { desktop_macos_h.events_pressed_mouse_buttons() })
        }

        public fun pressedModifiers(): KeyModifiersSet {
            return KeyModifiersSet(ffiDownCall { desktop_macos_h.events_pressed_modifiers() })
        }

        public fun cursorLocationInScreen(): LogicalPoint {
            return Arena.ofConfined().use { arena ->
                LogicalPoint.fromNative(ffiDownCall { desktop_macos_h.events_cursor_location_in_screen(arena) })
            }
        }
    }

    public data class KeyDown(
        override val windowId: WindowId,
        val keyCode: KeyCode,
        val typedCharacters: String,
        val key: String,
        val keyWithModifiers: String,
        val modifiers: KeyModifiersSet,
        val isRepeat: Boolean,
        val mightHaveKeyEquivalent: Boolean,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class KeyUp(
        override val windowId: WindowId,
        val keyCode: KeyCode,
        val typedCharacters: String,
        val key: String,
        val keyWithModifiers: String,
        val modifiers: KeyModifiersSet,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class ModifiersChanged(
        override val windowId: WindowId,
        val modifiers: KeyModifiersSet,
        val keyCode: KeyCode,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class MouseMoved(
        override val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class MouseDragged(
        override val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class MouseEntered(
        override val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class MouseExited(
        override val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class MouseUp(
        override val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val clickCount: Long,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class MouseDown(
        override val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val clickCount: Long,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class ScrollWheel(
        override val windowId: WindowId,
        val scrollingDeltaX: LogicalPixels,
        val scrollingDeltaY: LogicalPixels,
        val hasPreciseScrillingDeltas: Boolean,
        val isDirectionInverted: Boolean,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event(), WindowEvent

    public data class WindowScreenChange(
        override val windowId: WindowId,
        val newScreenId: ScreenId,
    ) : Event(), WindowEvent

    public data class WindowResize(
        override val windowId: WindowId,
        val size: LogicalSize,
    ) : Event(), WindowEvent

    public data class WindowMove(
        override val windowId: WindowId,
        val origin: LogicalPoint,
    ) : Event(), WindowEvent

    public data class WindowFocusChange(
        override val windowId: WindowId,
        val isKeyWindow: Boolean,
        val isMainWindow: Boolean,
    ) : Event(), WindowEvent

    public data class WindowFullScreenToggle(
        override val windowId: WindowId,
        val isFullScreen: Boolean,
    ) : Event(), WindowEvent

    public data object DisplayConfigurationChange : Event()

    public data object ApplicationDidFinishLaunching : Event()

    public data class WindowCloseRequest(override val windowId: WindowId) : Event(), WindowEvent

    public data class ApplicationAppearanceChange(val newAppearance: Appearance) : Event()
}

internal fun Event.Companion.fromNative(s: MemorySegment): Event {
    return when (NativeEvent.tag(s)) {
        desktop_macos_h.NativeEvent_KeyDown() -> {
            val nativeEvent = NativeEvent.key_down(s)
            Event.KeyDown(
                windowId = NativeKeyDownEvent.window_id(nativeEvent),
                keyCode = KeyCode.fromNative(NativeKeyDownEvent.code(nativeEvent)),
                typedCharacters = NativeKeyDownEvent.characters(nativeEvent).getUtf8String(0),
                key = NativeKeyDownEvent.key(nativeEvent).getUtf8String(0),
                keyWithModifiers = NativeKeyDownEvent.key_with_modifiers(nativeEvent).getUtf8String(0),
                modifiers = KeyModifiersSet(NativeKeyDownEvent.modifiers(nativeEvent)),
                isRepeat = NativeKeyDownEvent.is_repeat(nativeEvent),
                mightHaveKeyEquivalent = NativeKeyDownEvent.might_have_key_equivalent(nativeEvent),
                timestamp = Timestamp(NativeKeyDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_KeyUp() -> {
            val nativeEvent = NativeEvent.key_up(s)
            Event.KeyUp(
                windowId = NativeKeyUpEvent.window_id(nativeEvent),
                typedCharacters = NativeKeyUpEvent.characters(nativeEvent).getUtf8String(0),
                key = NativeKeyUpEvent.key(nativeEvent).getUtf8String(0),
                keyWithModifiers = NativeKeyUpEvent.key_with_modifiers(nativeEvent).getUtf8String(0),
                modifiers = KeyModifiersSet(NativeKeyUpEvent.modifiers(nativeEvent)),
                keyCode = KeyCode.fromNative(NativeKeyUpEvent.code(nativeEvent)),
                timestamp = Timestamp(NativeKeyUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_ModifiersChanged() -> {
            val nativeEvent = NativeEvent.modifiers_changed(s)
            Event.ModifiersChanged(
                windowId = NativeModifiersChangedEvent.window_id(nativeEvent),
                modifiers = KeyModifiersSet(NativeModifiersChangedEvent.modifiers(nativeEvent)),
                keyCode = KeyCode.fromNative(NativeModifiersChangedEvent.code(nativeEvent)),
                timestamp = Timestamp(NativeModifiersChangedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_MouseMoved() -> {
            val nativeEvent = NativeEvent.mouse_moved(s)
            Event.MouseMoved(
                windowId = NativeMouseMovedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseMovedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseMovedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_MouseDragged() -> {
            val nativeEvent = NativeEvent.mouse_dragged(s)
            Event.MouseDragged(
                windowId = NativeMouseDraggedEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseDraggedEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDraggedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseDraggedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_MouseEntered() -> {
            val nativeEvent = NativeEvent.mouse_entered(s)
            Event.MouseEntered(
                windowId = NativeMouseEnteredEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseEnteredEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseEnteredEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_MouseExited() -> {
            val nativeEvent = NativeEvent.mouse_exited(s)
            Event.MouseExited(
                windowId = NativeMouseExitedEvent.window_id(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeMouseExitedEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeMouseExitedEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_MouseUp() -> {
            val nativeEvent = NativeEvent.mouse_up(s)
            Event.MouseUp(
                windowId = NativeMouseUpEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseUpEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseUpEvent.location_in_window(nativeEvent)),
                clickCount = NativeMouseUpEvent.click_count(nativeEvent),
                timestamp = Timestamp(NativeMouseUpEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_MouseDown() -> {
            val nativeEvent = NativeEvent.mouse_down(s)
            Event.MouseDown(
                windowId = NativeMouseDownEvent.window_id(nativeEvent),
                button = MouseButton(NativeMouseDownEvent.button(nativeEvent)),
                locationInWindow = LogicalPoint.fromNative(NativeMouseDownEvent.location_in_window(nativeEvent)),
                clickCount = NativeMouseUpEvent.click_count(nativeEvent),
                timestamp = Timestamp(NativeMouseDownEvent.timestamp(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_ScrollWheel() -> {
            val nativeEvent = NativeEvent.scroll_wheel(s)
            Event.ScrollWheel(
                windowId = NativeScrollWheelEvent.window_id(nativeEvent),
                scrollingDeltaX = NativeScrollWheelEvent.scrolling_delta_x(nativeEvent),
                scrollingDeltaY = NativeScrollWheelEvent.scrolling_delta_y(nativeEvent),
                hasPreciseScrillingDeltas = NativeScrollWheelEvent.has_precise_scrolling_deltas(nativeEvent),
                isDirectionInverted = NativeScrollWheelEvent.is_direction_inverted(nativeEvent),
                locationInWindow = LogicalPoint.fromNative(NativeScrollWheelEvent.location_in_window(nativeEvent)),
                timestamp = Timestamp(NativeScrollWheelEvent.timestamp(nativeEvent)),
            )
        } desktop_macos_h.NativeEvent_WindowScreenChange() -> {
            val nativeEvent = NativeEvent.window_screen_change(s)
            Event.WindowScreenChange(
                windowId = NativeWindowScreenChangeEvent.window_id(nativeEvent),
                newScreenId = NativeWindowScreenChangeEvent.new_screen_id(nativeEvent),
            )
        }
        desktop_macos_h.NativeEvent_WindowResize() -> {
            val nativeEvent = NativeEvent.window_resize(s)
            Event.WindowResize(
                windowId = NativeWindowResizeEvent.window_id(nativeEvent),
                size = LogicalSize.fromNative(NativeWindowResizeEvent.size(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_WindowMove() -> {
            val nativeEvent = NativeEvent.window_move(s)
            Event.WindowMove(
                windowId = NativeWindowMoveEvent.window_id(nativeEvent),
                origin = LogicalPoint.fromNative(NativeWindowMoveEvent.origin(nativeEvent)),
            )
        }
        desktop_macos_h.NativeEvent_WindowFocusChange() -> {
            val nativeEvent = NativeEvent.window_focus_change(s)
            Event.WindowFocusChange(
                windowId = NativeWindowFocusChangeEvent.window_id(nativeEvent),
                isKeyWindow = NativeWindowFocusChangeEvent.is_key(nativeEvent),
                isMainWindow = NativeWindowFocusChangeEvent.is_main(nativeEvent),
            )
        }
        desktop_macos_h.NativeEvent_WindowCloseRequest() -> {
            val nativeEvent = NativeEvent.window_close_request(s)
            Event.WindowCloseRequest(
                windowId = NativeWindowCloseRequestEvent.window_id(nativeEvent),
            )
        }
        desktop_macos_h.NativeEvent_DisplayConfigurationChange() -> {
            Event.DisplayConfigurationChange
        }
        desktop_macos_h.NativeEvent_ApplicationDidFinishLaunching() -> {
            Event.ApplicationDidFinishLaunching
        }
        desktop_macos_h.NativeEvent_WindowFullScreenToggle() -> {
            val nativeEvent = NativeEvent.window_full_screen_toggle(s)
            Event.WindowFullScreenToggle(
                windowId = NativeWindowFullScreenToggleEvent.window_id(nativeEvent),
                isFullScreen = NativeWindowFullScreenToggleEvent.is_full_screen(nativeEvent),
            )
        }
        desktop_macos_h.NativeEvent_ApplicationAppearanceChange() -> {
            val nativeEvent = NativeEvent.application_appearance_change(s)
            Event.ApplicationAppearanceChange(
                newAppearance = Appearance.fromNative(NativeApplicationAppearanceChangeEvent.new_appearance(nativeEvent)),
            )
        }
        else -> {
            error("Unexpected Event tag")
        }
    }
}
