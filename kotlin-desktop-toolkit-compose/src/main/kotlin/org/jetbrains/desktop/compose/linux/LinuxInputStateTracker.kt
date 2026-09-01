@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.linux

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
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.LogicalPoint

/**
 * Turns the stream of Wayland input events into the pointer and key events a `ComposeScene` expects.
 *
 * Compose needs more than a translation of each event: it wants the *state* around it — which buttons are
 * held, which modifiers are down, whether the pointer is inside the surface at all — reported on every
 * event, because that is what it tracks drags and hover with. The compositor reports transitions, not
 * state, so this class holds the state in between.
 *
 * What it does beyond a one-to-one mapping:
 *  - pointer presence is tracked from Enter/Leave, which on Wayland is the only source of truth: a
 *    surface never learns about a pointer that is over a different one;
 *  - the pressed buttons are cleared on Leave, because an interactive move or resize grabs the pointer
 *    and the matching button release is delivered to the compositor rather than to us;
 *  - modifiers are held across a Leave as long as the surface still has keyboard focus, which on Wayland
 *    is a separate thing from pointer focus;
 *  - scroll deltas are turned into dp, distinguishing a discrete wheel notch from a smooth touchpad
 *    delta, and Shift swaps the axes;
 *  - a synthetic Move can be replayed after a relayout, so hover state follows content that moved out
 *    from under a stationary pointer.
 *
 * Unlike macOS, no synthetic key events have to be invented for the modifiers: the compositor delivers a
 * real KeyDown/KeyUp for `Shift_L` and friends alongside the `ModifiersChanged` that carries the new mask.
 */
internal class LinuxInputStateTracker(
    private val inputModeManager: InputModeManager,
    private val sendPointerEvent: (
        eventType: PointerEventType,
        position: Offset,
        scrollDelta: Offset,
        timeMillis: Long,
        buttons: PointerButtons,
        modifiers: PointerKeyboardModifiers,
        nativeEvent: Any?,
        button: PointerButton?,
    ) -> Unit,
    private val sendKeyEvent: (KeyEvent) -> Boolean,
) {
    private var isPointerInWindow: Boolean = false
    private var pointerPosition: Offset? = null
    private var lastEventTimeMillis: Long? = null
    private var pointerButtons = PointerButtons()
    private var hasKeyboardFocus: Boolean = false

    /**
     * Bumped by every real event so that a synthetic post-relayout Move prepared before it is dropped: by
     * the time it would be sent, the real event has already established a newer state.
     */
    private var syntheticMoveGeneration: Long = 0

    /** Reported to Compose through `WindowInfo.keyboardModifiers`, hence observable. */
    var keyboardModifiers: PointerKeyboardModifiers by mutableStateOf(PointerKeyboardModifiers())
        private set

    fun handleEvent(event: Event, density: Density): EventHandlerResult {
        syntheticMoveGeneration++
        return when (event) {
            is Event.MouseDown -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                isPointerInWindow = true
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                pointerButtons += event.button.toComposePointerButton()
                sendPointerEvent(
                    PointerEventType.Press,
                    event.locationInWindow.toPxOffset(density),
                    Offset.Zero,
                    lastEventTimeMillis(),
                    pointerButtons,
                    keyboardModifiers,
                    event,
                    event.button.toComposePointerButton(),
                )
                // Also offered as a key event, so that a shortcut bound to a side button is reachable.
                sendKeyEvent(event.button.toComposeKeyEvent(KeyEventType.KeyDown, keyboardModifiers, event))
                EventHandlerResult.Stop
            }

            is Event.MouseUp -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                // A native grab -- an interactive move, resize or drag -- takes the pointer and leaves
                // `MouseExited` behind, so the release can be the first event back without an intervening
                // `MouseEntered`. Without this the sends below drop out and hover stays stale.
                isPointerInWindow = true
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                pointerButtons -= event.button.toComposePointerButton()
                sendPointerEvent(
                    PointerEventType.Release,
                    event.locationInWindow.toPxOffset(density),
                    Offset.Zero,
                    lastEventTimeMillis(),
                    pointerButtons,
                    keyboardModifiers,
                    event,
                    event.button.toComposePointerButton(),
                )
                sendKeyEvent(event.button.toComposeKeyEvent(KeyEventType.KeyUp, keyboardModifiers, event))
                EventHandlerResult.Stop
            }

            is Event.MouseMoved -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                // The compositor only sends motion for the surface the pointer is actually over, so
                // unlike macOS there is nothing to second-guess here.
                isPointerInWindow = true
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                EventHandlerResult.Stop
            }

            is Event.MouseEntered -> {
                startPointerEvent(currentTimeMillis())
                isPointerInWindow = true
                movePointerTo(event.locationInWindow, PointerEventType.Enter, event, density)
                EventHandlerResult.Stop
            }

            is Event.MouseExited -> {
                startPointerEvent(currentTimeMillis())
                isPointerInWindow = false
                // An interactive move or resize grabs the pointer, and the compositor sends Leave when the
                // grab starts and keeps the matching button release to itself; a button not cleared here
                // would stay pressed forever.
                pointerButtons = PointerButtons()
                // Modifiers are a keyboard thing, and keyboard focus does not follow the pointer on
                // Wayland — they only have to be dropped when this surface has no keyboard focus either.
                if (!hasKeyboardFocus) {
                    keyboardModifiers = PointerKeyboardModifiers()
                }
                movePointerTo(event.locationInWindow, PointerEventType.Exit, event, density)
                EventHandlerResult.Stop
            }

            is Event.ScrollWheel -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                sendPointerEvent(
                    PointerEventType.Scroll,
                    event.locationInWindow.toPxOffset(density),
                    event.scrollDelta(keyboardModifiers.isShiftPressed).toPxOffset(density),
                    lastEventTimeMillis(),
                    pointerButtons,
                    keyboardModifiers,
                    event,
                    null,
                )
                // Scrolling moves content under a stationary pointer, so hover state has to be redone.
                sendPointerEventWithCurrentState(PointerEventType.Move)
                EventHandlerResult.Stop
            }

            is Event.KeyDown -> {
                lastEventTimeMillis = currentTimeMillis()
                dispatchKeyEvent(
                    KeyEvent(
                        key = event.key.toComposeKey(),
                        type = KeyEventType.KeyDown,
                        codePoint = event.codePoint(keyboardModifiers),
                        isCtrlPressed = keyboardModifiers.isCtrlPressed,
                        isMetaPressed = keyboardModifiers.isMetaPressed,
                        isAltPressed = keyboardModifiers.isAltPressed,
                        isShiftPressed = keyboardModifiers.isShiftPressed,
                        nativeEvent = event,
                    ),
                )
            }

            is Event.KeyUp -> {
                lastEventTimeMillis = currentTimeMillis()
                dispatchKeyEvent(
                    KeyEvent(
                        key = event.key.toComposeKey(),
                        type = KeyEventType.KeyUp,
                        isCtrlPressed = keyboardModifiers.isCtrlPressed,
                        isMetaPressed = keyboardModifiers.isMetaPressed,
                        isAltPressed = keyboardModifiers.isAltPressed,
                        isShiftPressed = keyboardModifiers.isShiftPressed,
                        nativeEvent = event,
                    ),
                )
            }

            is Event.ModifiersChanged -> {
                keyboardModifiers = event.modifiers.toPointerKeyboardModifiers()
                if (inputModeManager.inputMode == InputMode.Touch) {
                    // Hover feedback often depends on the modifiers (a Ctrl-clickable link, say).
                    sendPointerEventWithCurrentState(PointerEventType.Move)
                }
                EventHandlerResult.Continue
            }

            is Event.WindowKeyboardEnter -> {
                hasKeyboardFocus = true
                EventHandlerResult.Continue
            }

            is Event.WindowKeyboardLeave -> {
                hasKeyboardFocus = false
                // Every pressed modifier has to be released when focus goes away, or it sticks forever:
                // the release is delivered to whoever holds focus by then, not to us.
                if (!isPointerInWindow) {
                    keyboardModifiers = PointerKeyboardModifiers()
                }
                EventHandlerResult.Continue
            }

            else -> EventHandlerResult.Continue
        }
    }

    private fun startPointerEvent(timeMillis: Long) {
        lastEventTimeMillis = timeMillis
        // Any pointer activity means the user is not navigating by keyboard any more.
        if (inputModeManager.inputMode != InputMode.Touch) {
            inputModeManager.requestInputMode(InputMode.Touch)
        }
    }

    private fun lastEventTimeMillis(): Long = lastEventTimeMillis ?: System.currentTimeMillis()

    private fun currentTimeMillis(): Long = System.currentTimeMillis().also { lastEventTimeMillis = it }

    private fun dispatchKeyEvent(keyEvent: KeyEvent): EventHandlerResult {
        return if (sendKeyEvent(keyEvent)) EventHandlerResult.Stop else EventHandlerResult.Continue
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
     * Prepares a pointer event that refreshes hit testing under a stationary pointer after the content was
     * laid out again, or `null` when there is nothing to refresh.
     *
     * Deliberately window-local: it replays the position we already know rather than asking the platform
     * where the pointer is, which Wayland cannot answer at all.
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
}

internal data class PendingSyntheticMove(
    val generation: Long,
    val type: PointerEventType,
)

/**
 * The codepoint the key produced, or 0 when it produced no text of its own.
 *
 * A Ctrl chord is normalised to the matching ISO control character (Ctrl+F becomes 0x06), because that is
 * what the other backends report and what Compose's shortcut handling expects; AltGr, which arrives as
 * Ctrl+Alt, is deliberately exempt because it produces real text.
 */
private fun Event.KeyDown.codePoint(modifiers: PointerKeyboardModifiers): Int {
    val raw = characters?.firstCodePointOrNull() ?: 0
    return if (modifiers.isCtrlPressed && !modifiers.isAltPressed && raw in 0x40..0x7E) raw and 0x1F else raw
}

/**
 * Scroll deltas as Compose wants them, in dp of content movement.
 *
 * A discrete wheel reports `wheelValue120` in 120ths of a notch and no useful delta, so a notch is turned
 * into a fixed 100 dp; a touchpad or a high-resolution wheel reports a smooth delta in logical pixels,
 * which only needs scaling up. Holding Shift swaps the axes, so a plain vertical wheel scrolls sideways.
 */
private fun Event.ScrollWheel.scrollDelta(isShiftPressed: Boolean): DpOffset {
    val delta = if (verticalScroll.wheelValue120 != 0 || horizontalScroll.wheelValue120 != 0) {
        DpOffset(
            (horizontalScroll.wheelValue120 / 120f * 100f).dp,
            (verticalScroll.wheelValue120 / 120f * 100f).dp,
        )
    } else {
        DpOffset(
            (horizontalScroll.delta.rawLogical * 15).dp,
            (verticalScroll.delta.rawLogical * 15).dp,
        )
    }
    return if (isShiftPressed) DpOffset(delta.y, delta.x) else delta
}

private fun org.jetbrains.desktop.linux.MouseButton.toComposeKeyEvent(
    type: KeyEventType,
    modifiers: PointerKeyboardModifiers,
    nativeEvent: Any,
): KeyEvent = KeyEvent(
    key = toComposeKey(),
    type = type,
    codePoint = 0,
    isCtrlPressed = modifiers.isCtrlPressed,
    isMetaPressed = modifiers.isMetaPressed,
    isAltPressed = modifiers.isAltPressed,
    isShiftPressed = modifiers.isShiftPressed,
    nativeEvent = nativeEvent,
)

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
