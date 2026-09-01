@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.gtk

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
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.EventHandlerResult
import org.jetbrains.desktop.gtk.LogicalPoint

/**
 * Turns the stream of GTK input events into the pointer and key events a `ComposeScene` expects.
 *
 * Compose needs more than a translation of each event: it wants the *state* around it — which buttons are
 * held, which modifiers are down, whether the pointer is inside the window at all — reported on every
 * event, because that is what it tracks drags and hover with. GTK reports transitions, not state, so this
 * class holds the state in between.
 *
 * What it does beyond a one-to-one mapping:
 *  - pointer presence is tracked from Enter/Leave, which is the only source of truth: a window never
 *    learns about a pointer that is over a different one;
 *  - the pressed buttons are cleared on Leave, because a native grab — an interactive resize, or a drag —
 *    takes the pointer and the matching button release is delivered to GTK rather than to us;
 *  - modifiers are held across a Leave as long as the window still has keyboard focus, which is a
 *    separate thing from pointer focus;
 *  - scroll deltas are turned into dp, distinguishing a discrete wheel notch from a smooth touchpad
 *    delta, and Shift swaps the axes;
 *  - a synthetic Move can be replayed after a relayout, so hover state follows content that moved out
 *    from under a stationary pointer.
 *
 * Two GTK events carry less than their Wayland counterparts and are worked around here: `MouseExited` and
 * `ScrollWheel` report no pointer position, so both reuse the last one this class saw.
 *
 * Unlike macOS, no synthetic key events have to be invented for the modifiers: GTK delivers a real
 * KeyDown/KeyUp for `Shift_L` and friends alongside the `ModifiersChanged` that carries the new mask.
 */
internal class GtkInputStateTracker(
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
                POINTER_EVENT_RESULT
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
                POINTER_EVENT_RESULT
            }

            is Event.MouseMoved -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                // GTK only sends motion for the window the pointer is actually over, so unlike macOS
                // there is nothing to second-guess here.
                isPointerInWindow = true
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                POINTER_EVENT_RESULT
            }

            is Event.MouseEntered -> {
                startPointerEvent(currentTimeMillis())
                isPointerInWindow = true
                movePointerTo(event.locationInWindow, PointerEventType.Enter, event, density)
                POINTER_EVENT_RESULT
            }

            is Event.MouseExited -> {
                startPointerEvent(currentTimeMillis())
                // GTK's Leave carries no position, so the Exit replays the last one we saw. It is sent
                // before the state below is cleared, because Compose reads it as "hover ended with these
                // buttons held" and a drag that leaves the window has to keep them.
                // Forced true first: a duplicate Leave, or one arriving after a grab already cleared the
                // flag, would otherwise be swallowed by the guard in `sendPointerEventWithCurrentState`
                // and leave the last hovered element highlighted for good.
                isPointerInWindow = true
                sendPointerEventWithCurrentState(PointerEventType.Exit, event)
                isPointerInWindow = false
                // A native grab — an interactive resize, or a drag — takes the pointer when the grab
                // starts and keeps the matching button release to itself; a button not cleared here would
                // stay pressed forever.
                pointerButtons = PointerButtons()
                POINTER_EVENT_RESULT
            }

            is Event.ScrollWheel -> {
                // The end-of-gesture event carries no delta of its own and Compose has nothing to do with
                // it; scrolling by zero would only cost a pointless hover pass.
                if (event.isStop) {
                    return POINTER_EVENT_RESULT
                }
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                // GTK's scroll event carries no position either, so it scrolls wherever the pointer was
                // last seen. Nothing to scroll before the pointer has ever been in the window.
                val position = pointerPosition ?: return EventHandlerResult.Continue
                sendPointerEvent(
                    PointerEventType.Scroll,
                    position,
                    event.scrollDelta(keyboardModifiers.isShiftPressed).toPxOffset(density),
                    lastEventTimeMillis(),
                    pointerButtons,
                    keyboardModifiers,
                    event,
                    null,
                )
                // Scrolling moves content under a stationary pointer, so hover state has to be redone.
                sendPointerEventWithCurrentState(PointerEventType.Move)
                POINTER_EVENT_RESULT
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
     * where the pointer is, which this backend has no call for.
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

    private companion object {
        /**
         * What every pointer branch answers, and deliberately not `Stop`.
         *
         * The toolkit hangs its pointer controllers off the window's content widget in GTK's *capture*
         * phase, so `Stop` does not merely mean "Compose handled this" — it keeps the event from ever
         * reaching the widgets underneath. Over a [org.jetbrains.desktop.compose.WindowDecoration.CustomTitleBar]
         * those widgets are GTK's own: the window buttons and the handle that drags and maximizes the
         * window. Stopping there would leave such a window unmovable with dead buttons.
         *
         * Letting the event through costs nothing elsewhere — the content widget has no interactive GTK
         * children — and it is why content must not add titlebar gestures of its own on this backend; see
         * [org.jetbrains.desktop.compose.ComposeWindow.handlesCustomTitleBarGestures].
         */
        val POINTER_EVENT_RESULT = EventHandlerResult.Continue
    }

    private fun sendPointerEventWithCurrentState(type: PointerEventType, nativeEvent: Event? = null) {
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
            nativeEvent,
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
 * GTK reports one delta per axis and says whether it is smooth. A touchpad or high-resolution wheel gives
 * a delta in logical pixels, which only needs scaling up; a discrete wheel gives whole notches, so a notch
 * becomes a fixed 100 dp. Holding Shift swaps the axes, so a plain vertical wheel scrolls sideways.
 */
private fun Event.ScrollWheel.scrollDelta(isShiftPressed: Boolean): DpOffset {
    val scale = if (isSmoothScroll) 15f else 100f
    val delta = DpOffset(
        (scrollingDeltaX.rawLogical.toFloat() * scale).dp,
        (scrollingDeltaY.rawLogical.toFloat() * scale).dp,
    )
    return if (isShiftPressed) DpOffset(delta.y, delta.x) else delta
}

private fun org.jetbrains.desktop.gtk.MouseButton.toComposeKeyEvent(
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
