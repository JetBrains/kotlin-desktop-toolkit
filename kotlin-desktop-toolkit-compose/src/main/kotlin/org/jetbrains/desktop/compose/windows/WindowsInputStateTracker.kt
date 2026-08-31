@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.windows

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.InputMode
import androidx.compose.ui.input.InputModeManager
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.pointer.PointerButton
import androidx.compose.ui.input.pointer.PointerButtons
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.input.pointer.areAnyPressed
import androidx.compose.ui.input.pointer.isAltPressed
import androidx.compose.ui.input.pointer.isBack
import androidx.compose.ui.input.pointer.isBackPressed
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isForward
import androidx.compose.ui.input.pointer.isForwardPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.isPrimary
import androidx.compose.ui.input.pointer.isPrimaryPressed
import androidx.compose.ui.input.pointer.isSecondary
import androidx.compose.ui.input.pointer.isSecondaryPressed
import androidx.compose.ui.input.pointer.isShiftPressed
import androidx.compose.ui.input.pointer.isTertiary
import androidx.compose.ui.input.pointer.isTertiaryPressed
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.PointerButton as Win32PointerButton

/**
 * Turns the stream of win32 input events into the pointer and key events a `ComposeScene` expects.
 *
 * Compose needs more than a translation of each message: it wants the *state* around it — which buttons
 * are held, which modifiers are down, whether the pointer is inside the window at all — reported on
 * every event, because that is what it tracks drags and hover with. Windows reports transitions, so this
 * class holds the state in between.
 *
 * Three things here are win32-specific rather than shared with the macOS and Linux trackers:
 *  - modifiers are read from the keyboard on every event ([currentKeyboardModifiers]) instead of being
 *    tracked, because Windows has no "modifiers changed" message to track them from; the mask is still
 *    re-published on a modifier key event, which is what `WindowInfo.keyboardModifiers` reads;
 *  - Windows stops delivering pointer movements once the cursor leaves the window, so an exit is
 *    followed by a synthetic movement placed just outside — see [sendSyntheticPointerMovedOutsideWindow];
 *  - a modifier held while the window is deactivated would stick forever, so activation is where the
 *    mask is re-snapshotted and deactivation is where it is cleared.
 *
 * On what it reports back to the toolkit: for key events the answer is real — `sendKeyEvent` says
 * whether anything handled the key, and an unhandled system key still has to reach `TranslateMessage`.
 * For pointer events, `ComposeScene`'s `PointerEventResult` members are internal to compose-ui, so a
 * host outside the Compose modules cannot read them; what decides the answer instead is whether the
 * event came from the non-client area. A press on a custom titlebar's drag region arrives with
 * `nonClientArea` set, and reporting it as handled would keep `DefWindowProc` from running the window's
 * own move loop — so those are passed on, and everything from the client area is claimed.
 */
internal class WindowsInputStateTracker(
    private val inputModeManager: InputModeManager,
    private val sendPointerEvent: (
        eventType: PointerEventType,
        position: Offset,
        scrollDelta: Offset,
        timeMillis: Long,
        buttons: PointerButtons,
        keyboardModifiers: PointerKeyboardModifiers,
        nativeEvent: Any?,
        button: PointerButton?,
    ) -> Unit,
    private val sendKeyEvent: (KeyEvent) -> Boolean,
) {
    private var isPointerInWindow: Boolean = false
    private var pointerPosition: Offset? = null
    private var lastEventTimeMillis: Long? = null
    private var pointerButtons = PointerButtons()

    /**
     * Bumped by every real event so that a synthetic post-relayout Move prepared before it is dropped:
     * by the time it would be sent, the real event has already established a newer state.
     */
    private var syntheticMoveGeneration: Long = 0

    /** Reported to Compose through `WindowInfo.keyboardModifiers`, hence observable. */
    var keyboardModifiers: PointerKeyboardModifiers by mutableStateOf(PointerKeyboardModifiers())
        private set

    fun handleEvent(event: Event, density: Density): EventHandlerResult {
        syntheticMoveGeneration++
        return when (event) {
            is Event.PointerDown -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                val button = event.button.toComposePointerButton()
                pointerButtons += button
                keyboardModifiers = currentKeyboardModifiers()
                sendPointerEvent(
                    PointerEventType.Press,
                    event.locationInWindow.toPxOffset(density),
                    Offset.Zero,
                    lastEventTimeMillis(),
                    pointerButtons,
                    keyboardModifiers,
                    event,
                    button,
                )
                // Also offered as a key event, so that a shortcut bound to a side button is reachable.
                sendKeyEvent(event.button.toComposeKeyEvent(KeyEventType.KeyDown, keyboardModifiers, event))
                resultFor(nonClientArea = event.nonClientArea)
            }

            is Event.PointerUp -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                val button = event.button.toComposePointerButton()
                pointerButtons -= button
                keyboardModifiers = currentKeyboardModifiers()
                sendPointerEvent(
                    PointerEventType.Release,
                    event.locationInWindow.toPxOffset(density),
                    Offset.Zero,
                    lastEventTimeMillis(),
                    pointerButtons,
                    keyboardModifiers,
                    event,
                    button,
                )
                sendKeyEvent(event.button.toComposeKeyEvent(KeyEventType.KeyUp, keyboardModifiers, event))
                resultFor(nonClientArea = event.nonClientArea)
            }

            is Event.PointerUpdated -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                keyboardModifiers = currentKeyboardModifiers()
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                resultFor(nonClientArea = event.nonClientArea)
            }

            is Event.PointerEntered -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                isPointerInWindow = true
                movePointerTo(event.locationInWindow, PointerEventType.Enter, event, density)
                EventHandlerResult.Stop
            }

            is Event.PointerExited -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                isPointerInWindow = false
                val lastPositionInWindow = pointerPosition
                movePointerTo(event.locationInWindow, PointerEventType.Exit, event, density)
                // Skipped while a button is held: the pointer is captured, and the drag has to keep
                // seeing real positions.
                if (!pointerButtons.areAnyPressed) {
                    sendSyntheticPointerMovedOutsideWindow(lastPositionInWindow, event, density)
                }
                EventHandlerResult.Stop
            }

            is Event.ScrollWheelX -> {
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                keyboardModifiers = currentKeyboardModifiers()
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                sendScroll(event.locationInWindow, horizontalScrollDelta(event.scrollingDelta), event, density)
                EventHandlerResult.Stop
            }

            is Event.ScrollWheelY -> {
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                keyboardModifiers = currentKeyboardModifiers()
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                val delta = verticalScrollDelta(event.scrollingDelta, keyboardModifiers.isShiftPressed)
                sendScroll(event.locationInWindow, delta, event, density)
                EventHandlerResult.Stop
            }

            is Event.KeyDown -> {
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                val modifiers = currentKeyboardModifiers()
                val handled = sendKeyEvent(
                    KeyEvent(
                        key = event.virtualKey.toComposeKey(),
                        type = KeyEventType.KeyDown,
                        codePoint = event.toUnicode().firstOrNull()?.code ?: 0,
                        isCtrlPressed = modifiers.isCtrlPressed,
                        isMetaPressed = modifiers.isMetaPressed,
                        isAltPressed = modifiers.isAltPressed,
                        isShiftPressed = modifiers.isShiftPressed,
                        nativeEvent = event,
                    ),
                )
                // After dispatch: the event itself reports the state before the key was applied, so a
                // handler asking for the modifiers of its own Shift press would otherwise not see it.
                if (event.virtualKey.isModifier()) {
                    keyboardModifiers = modifiers
                }
                if (handled) EventHandlerResult.Stop else EventHandlerResult.Continue
            }

            is Event.KeyUp -> {
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                val modifiers = currentKeyboardModifiers()
                val handled = sendKeyEvent(
                    KeyEvent(
                        key = event.virtualKey.toComposeKey(),
                        type = KeyEventType.KeyUp,
                        isCtrlPressed = modifiers.isCtrlPressed,
                        isMetaPressed = modifiers.isMetaPressed,
                        isAltPressed = modifiers.isAltPressed,
                        isShiftPressed = modifiers.isShiftPressed,
                        nativeEvent = event,
                    ),
                )
                if (event.virtualKey.isModifier()) {
                    keyboardModifiers = modifiers
                }
                if (handled) EventHandlerResult.Stop else EventHandlerResult.Continue
            }

            is Event.WindowActivated -> {
                // Every pressed modifier has to be released when the window is deactivated, or it sticks
                // forever: hold Alt, press Tab, and the release is delivered to whoever has focus by then.
                keyboardModifiers = if (event.active) currentKeyboardModifiers() else PointerKeyboardModifiers()
                EventHandlerResult.Continue
            }

            else -> EventHandlerResult.Continue
        }
    }

    /**
     * A press that the platform owns keeps its default handling: the drag region of a custom titlebar is
     * reported as non-client, and `DefWindowProc` is what moves the window when a press there goes
     * unhandled.
     */
    private fun resultFor(nonClientArea: Boolean): EventHandlerResult =
        if (nonClientArea) EventHandlerResult.Continue else EventHandlerResult.Stop

    private fun startPointerEvent(timeMillis: Long) {
        lastEventTimeMillis = timeMillis
        // Any pointer activity means the user is not navigating by keyboard any more, which is also what
        // brings a cursor hidden while they typed back.
        if (inputModeManager.inputMode != InputMode.Touch) {
            inputModeManager.requestInputMode(InputMode.Touch)
        }
    }

    private fun lastEventTimeMillis(): Long = lastEventTimeMillis ?: System.currentTimeMillis()

    private fun sendScroll(locationInWindow: LogicalPoint, delta: DpOffset, nativeEvent: Event, density: Density) {
        sendPointerEvent(
            PointerEventType.Scroll,
            locationInWindow.toPxOffset(density),
            delta.toPxOffset(density),
            lastEventTimeMillis(),
            pointerButtons,
            keyboardModifiers,
            nativeEvent,
            null,
        )
        // Scrolling moves content under a stationary pointer, so hover state has to be redone.
        sendPointerEventWithCurrentState(PointerEventType.Move)
    }

    private fun movePointerTo(locationInWindow: LogicalPoint, eventType: PointerEventType, nativeEvent: Event, density: Density) {
        val previousPosition = pointerPosition
        val position = locationInWindow.toPxOffset(density)
        pointerPosition = position
        // A Move that does not move is noise; Enter and Exit matter regardless of position.
        if (previousPosition == position && eventType == PointerEventType.Move) {
            return
        }
        sendPointerEvent(
            eventType,
            position,
            Offset.Zero,
            lastEventTimeMillis(),
            pointerButtons,
            keyboardModifiers,
            nativeEvent,
            null,
        )
    }

    /**
     * Continues the cursor's last travel past the window edge and reports it as a Move.
     *
     * An Exit event on its own changes nothing about hover: Compose derives hover from which nodes a
     * position hits, and only a *later* event whose hit path no longer contains a node makes that node
     * hear about it. Windows delivers no movement at all once the cursor is outside the window, so on a
     * fast exit the last reported position can still sit on a hovered control and leave it stuck.
     */
    private fun sendSyntheticPointerMovedOutsideWindow(lastPositionInWindow: Offset?, nativeEvent: Event, density: Density) {
        val exitPosition = pointerPosition ?: return
        val outsidePosition = with(density) {
            syntheticExitPosition(exitPosition, lastPositionInWindow, minNudgePx = SYNTHETIC_EXIT_NUDGE.toPx())
        }
        pointerPosition = outsidePosition
        sendPointerEvent(
            PointerEventType.Move,
            outsidePosition,
            Offset.Zero,
            lastEventTimeMillis(),
            pointerButtons,
            keyboardModifiers,
            nativeEvent,
            null,
        )
    }

    /**
     * Prepares a pointer event that refreshes hit testing under a stationary pointer after the content
     * was laid out again, or `null` when there is nothing to refresh.
     */
    fun prepareSyntheticMoveAfterRelayout(): PendingSyntheticMove? {
        if (pointerPosition == null) {
            return null
        }
        val type = when {
            pointerButtons.areAnyPressed -> PointerEventType.Move
            !isPointerInWindow -> return null
            inputModeManager.inputMode == InputMode.Touch -> PointerEventType.Move
            else -> PointerEventType.Exit
        }
        val generation = syntheticMoveGeneration + 1
        syntheticMoveGeneration = generation
        return PendingSyntheticMove(generation, type)
    }

    /** Sends [request] unless a real event has arrived since it was prepared. */
    fun sendSyntheticMoveIfCurrent(request: PendingSyntheticMove) {
        if (request.generation != syntheticMoveGeneration) {
            return
        }
        sendPointerEventWithCurrentState(request.type)
    }

    private fun sendPointerEventWithCurrentState(type: PointerEventType) {
        val position = pointerPosition
        if (!isPointerInWindow || position == null) {
            return
        }
        sendPointerEvent(
            type,
            position,
            Offset.Zero,
            lastEventTimeMillis(),
            pointerButtons,
            keyboardModifiers,
            null,
            null,
        )
    }

    private companion object {
        /**
         * How far past the window edge the synthetic "cursor left the window" movement is placed. A lower
         * bound on the continuation distance; overshooting only lands further outside, never back in.
         */
        val SYNTHETIC_EXIT_NUDGE = 32.dp
    }
}

internal data class PendingSyntheticMove(
    val generation: Long,
    val type: PointerEventType,
)

/**
 * Extends the pointer's last travel vector past the window edge, to at least [minNudgePx] so that a tiny
 * final movement still clears it. With no travel to extrapolate the point is pushed above the top edge,
 * which is always outside the root layout.
 */
internal fun syntheticExitPosition(exitPosition: Offset, lastPositionInWindow: Offset?, minNudgePx: Float): Offset {
    val travel = if (lastPositionInWindow != null) exitPosition - lastPositionInWindow else Offset.Zero
    val distance = travel.getDistance()
    return if (distance > 0f) {
        exitPosition + (travel / distance) * maxOf(distance, minNudgePx)
    } else {
        Offset(exitPosition.x, -minNudgePx)
    }
}

/** Horizontal wheel deltas need no inversion: the tilt already points the way the content should move. */
private fun horizontalScrollDelta(scrollingDelta: Int): DpOffset = DpOffset(scrollingDelta.toFloat().dp, 0.dp)

/**
 * The vertical wheel is inverted (content follows the fingers), and Shift redirects it onto the
 * horizontal axis — the standard Windows shift-scroll gesture.
 */
private fun verticalScrollDelta(scrollingDelta: Int, isShiftPressed: Boolean): DpOffset {
    val delta = (-scrollingDelta.toFloat()).dp
    return if (isShiftPressed) DpOffset(delta, 0.dp) else DpOffset(0.dp, delta)
}

private fun Win32PointerButton.toComposeKeyEvent(type: KeyEventType, modifiers: PointerKeyboardModifiers, nativeEvent: Event): KeyEvent =
    KeyEvent(
        key = toComposeKey(),
        type = type,
        isCtrlPressed = modifiers.isCtrlPressed,
        isMetaPressed = modifiers.isMetaPressed,
        isAltPressed = modifiers.isAltPressed,
        isShiftPressed = modifiers.isShiftPressed,
        nativeEvent = nativeEvent,
    )

private fun LogicalPoint.toPxOffset(density: Density): Offset = toDpOffset().toPxOffset(density)

// `PointerButtons.copy` is internal to compose-ui, so the set is rebuilt from its flags instead.
private operator fun PointerButtons.plus(other: PointerButton): PointerButtons = PointerButtons(
    isPrimaryPressed = isPrimaryPressed || other.isPrimary,
    isSecondaryPressed = isSecondaryPressed || other.isSecondary,
    isTertiaryPressed = isTertiaryPressed || other.isTertiary,
    isBackPressed = isBackPressed || other.isBack,
    isForwardPressed = isForwardPressed || other.isForward,
)

private operator fun PointerButtons.minus(other: PointerButton): PointerButtons = PointerButtons(
    isPrimaryPressed = isPrimaryPressed && !other.isPrimary,
    isSecondaryPressed = isSecondaryPressed && !other.isSecondary,
    isTertiaryPressed = isTertiaryPressed && !other.isTertiary,
    isBackPressed = isBackPressed && !other.isBack,
    isForwardPressed = isForwardPressed && !other.isForward,
)
