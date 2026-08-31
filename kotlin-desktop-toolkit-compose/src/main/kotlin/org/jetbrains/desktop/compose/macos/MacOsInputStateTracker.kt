@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.macos

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.InputMode
import androidx.compose.ui.input.InputModeManager
import androidx.compose.ui.input.key.Key
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
import androidx.compose.ui.input.pointer.isCapsLockOn
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isForward
import androidx.compose.ui.input.pointer.isForwardPressed
import androidx.compose.ui.input.pointer.isFunctionPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.isNumLockOn
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
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyModifiersSet
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.MouseButton
import org.jetbrains.desktop.macos.WindowEvent

/**
 * Turns the stream of macOS input events into the pointer and key events a `ComposeScene` expects.
 *
 * Compose needs more than a translation of each event: it wants the *state* around it — which buttons
 * are held, which modifiers are down, whether the pointer is inside the window at all — reported on
 * every event, because that is what it tracks drags and hover with. AppKit reports transitions, not
 * state, so this class holds the state in between.
 *
 * What it does beyond a one-to-one mapping:
 *  - pointer presence is tracked from Enter/Exit rather than from the pointer's position, because with
 *    overlapping windows a bounds check is not the same question;
 *  - modifier changes are expanded into synthetic key down/up events, which is the only way Compose
 *    hears about a bare Cmd press, and are cleared on focus loss so a held modifier cannot stick;
 *  - scroll deltas are scaled depending on whether the device reports precise deltas;
 *  - trackpad swipes become back/forward button clicks;
 *  - a synthetic Move can be replayed after a relayout, so hover state follows content that moved out
 *    from under a stationary pointer.
 *
 * On what it reports back to the toolkit: for key events the answer is real — `ComposeScene.sendKeyEvent`
 * says whether anything handled the key, and that decides whether AppKit still gets to run its own
 * menu-equivalent scan. For pointer events there is no such answer to give. `ComposeScene`'s
 * `PointerEventResult` members are internal to compose-ui, so a host outside the Compose modules cannot
 * read them; and the scene fills the whole content area anyway, which makes it the owner of every
 * pointer event the window receives. So a pointer event this class acted on is reported as handled, and
 * only one it deliberately dropped is passed on. (Today the macOS backend ignores the result for pointer
 * events entirely — it never forwards them to the next responder — so this is a statement of intent
 * rather than an observable behaviour.)
 */
internal class MacOsInputStateTracker(
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
    /**
     * With potentially overlapping windows this cannot be derived from the pointer position, so
     * Enter/Exit events are tracked instead.
     */
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

    fun handleEvent(event: WindowEvent, density: Density): EventHandlerResult {
        syntheticMoveGeneration++
        return when (event) {
            is Event.MouseDown -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
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
                sendKeyEvent(event.toComposeKeyEvent(keyboardModifiers))
                EventHandlerResult.Stop
            }

            is Event.MouseUp -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
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
                sendKeyEvent(event.toComposeKeyEvent(keyboardModifiers))
                EventHandlerResult.Stop
            }

            is Event.MouseMoved -> {
                // AppKit keeps delivering mouseMoved to the key window even when another window (or the
                // Dock) genuinely occludes the cursor. A Move only refines a position a real Enter has
                // already established, so without one there is nothing to trust it against.
                if (!isPointerInWindow) {
                    return EventHandlerResult.Continue
                }
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                pointerButtons = PointerButtons()
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                EventHandlerResult.Stop
            }

            is Event.MouseDragged -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                pointerButtons += event.button.toComposePointerButton()
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                EventHandlerResult.Stop
            }

            is Event.MouseEntered -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                isPointerInWindow = true
                movePointerTo(event.locationInWindow, PointerEventType.Enter, event, density)
                EventHandlerResult.Stop
            }

            is Event.MouseExited -> {
                startPointerEvent(event.timestamp.toDuration().inWholeMilliseconds)
                isPointerInWindow = false
                movePointerTo(event.locationInWindow, PointerEventType.Exit, event, density)
                EventHandlerResult.Stop
            }

            is Event.ScrollWheel -> {
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                sendPointerEvent(
                    PointerEventType.Scroll,
                    event.locationInWindow.toPxOffset(density),
                    event.scrollDelta().toPxOffset(density),
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
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                updateModifiers(event.modifiers, event)
                dispatchKeyEvent(event.toComposeKeyEvent())
            }

            is Event.KeyUp -> {
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                updateModifiers(event.modifiers, event)
                dispatchKeyEvent(event.toComposeKeyEvent())
            }

            is Event.ModifiersChanged -> {
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                updateModifiers(event.modifiers, event)
                if (inputModeManager.inputMode == InputMode.Touch) {
                    // Hover feedback often depends on the modifiers (a Cmd-clickable link, say).
                    sendPointerEventWithCurrentState(PointerEventType.Move)
                }
                EventHandlerResult.Continue
            }

            is Event.WindowFocusChange -> {
                if (event.isKeyWindow) {
                    updateModifiers(Event.pressedModifiers(), event)
                } else {
                    // Every pressed modifier has to be released when the window loses focus, or it
                    // sticks forever. To see why, hold Cmd, press Tab, then come back to the window:
                    // the Cmd release is delivered to whoever had focus by then, not to us.
                    updateModifiers(KeyModifiersSet.create(), event)
                }
                EventHandlerResult.Continue
            }

            is Event.Swipe -> {
                lastEventTimeMillis = event.timestamp.toDuration().inWholeMilliseconds
                movePointerTo(event.locationInWindow, PointerEventType.Move, event, density)
                // A vertical swipe maps onto no button, so it is left for whoever else wants it.
                val button = event.toBackForwardButton() ?: return EventHandlerResult.Continue
                sendSyntheticClick(button, event, event.locationInWindow.toPxOffset(density))
                EventHandlerResult.Stop
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

    private fun dispatchKeyEvent(keyEvent: KeyEvent): EventHandlerResult {
        // Stop claims the key as handled and keeps AppKit from running its own menu-equivalent scan;
        // Continue is what lets system shortcuts (Cmd+H, Cmd+Q, ...) still reach the native path even
        // though every key goes through Compose first.
        return if (sendKeyEvent(keyEvent)) EventHandlerResult.Stop else EventHandlerResult.Continue
    }

    /**
     * A press immediately followed by a release, for input that has no separate down and up — a
     * trackpad swipe mapped onto the back/forward buttons.
     */
    private fun sendSyntheticClick(button: MouseButton, event: Event.Swipe, position: Offset) {
        val pointerButton = button.toComposePointerButton()
        sendPointerEvent(
            PointerEventType.Press,
            position,
            Offset.Zero,
            lastEventTimeMillis(),
            pointerButtons + pointerButton,
            keyboardModifiers,
            event,
            pointerButton,
        )
        sendKeyEvent(button.toComposeKeyEvent(event, KeyEventType.KeyDown, keyboardModifiers))
        sendPointerEvent(
            PointerEventType.Release,
            position,
            Offset.Zero,
            lastEventTimeMillis(),
            pointerButtons,
            keyboardModifiers,
            event,
            pointerButton,
        )
        sendKeyEvent(button.toComposeKeyEvent(event, KeyEventType.KeyUp, keyboardModifiers))
    }

    private fun movePointerTo(locationInWindow: LogicalPoint, eventType: PointerEventType, nativeEvent: WindowEvent, density: Density) {
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

    private fun updateModifiers(modifiers: KeyModifiersSet, nativeEvent: WindowEvent) {
        val previous = keyboardModifiers
        keyboardModifiers = modifiers.toPointerKeyboardModifiers()
        if (previous == keyboardModifiers) {
            return
        }
        // Compose only learns about a modifier press from a key event, and macOS delivers none for
        // modifiers — `ModifiersChanged` reports the new mask instead — so the transitions are
        // reconstructed by diffing the two masks.
        for ((key, wasPressed, isPressed) in previous.transitionsTo(keyboardModifiers)) {
            if (wasPressed == isPressed) continue
            sendKeyEvent(
                KeyEvent(
                    key = key,
                    type = if (isPressed) KeyEventType.KeyDown else KeyEventType.KeyUp,
                    isCtrlPressed = keyboardModifiers.isCtrlPressed,
                    isMetaPressed = keyboardModifiers.isMetaPressed,
                    isAltPressed = keyboardModifiers.isAltPressed,
                    isShiftPressed = keyboardModifiers.isShiftPressed,
                    nativeEvent = nativeEvent,
                ),
            )
        }
    }

    /**
     * Prepares a pointer event that refreshes hit testing under a stationary pointer after the content
     * was laid out again, or `null` when there is nothing to refresh.
     *
     * Deliberately window-local: it replays the position we already know rather than asking the platform
     * where the pointer is, which not every platform can answer.
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

private fun LogicalPoint.toPxOffset(density: Density): Offset = toDpOffset().toPxOffset(density)

private fun Event.Swipe.toBackForwardButton(): MouseButton? = when {
    deltaX > 0.0 -> MouseButton.BACK
    deltaX < 0.0 -> MouseButton.FORWARD
    else -> null
}

/**
 * Scroll deltas as Compose wants them: in the direction the content should move, which on macOS is the
 * opposite of the wheel delta.
 *
 * A trackpad or a Magic Mouse reports precise, pixel-grained deltas that only need scaling down; a
 * classic wheel reports whole notches, which AppKit says to multiply by the line height — one dp per
 * notch is the placeholder for that.
 */
private fun Event.ScrollWheel.scrollDelta(): DpOffset {
    val scale = if (hasPreciseScrillingDeltas) 0.1f else 1f
    return DpOffset(
        (-scale * scrollingDeltaX).dp,
        (-scale * scrollingDeltaY).dp,
    )
}

private fun KeyModifiersSet.toPointerKeyboardModifiers(): PointerKeyboardModifiers {
    return PointerKeyboardModifiers(
        isCtrlPressed = control,
        isMetaPressed = command,
        isAltPressed = option,
        isShiftPressed = shift,
        isFunctionPressed = function,
        isCapsLockOn = capsLock,
        isNumLockOn = numericPad,
    )
}

/** One modifier's before/after state, for diffing two [PointerKeyboardModifiers]. */
private data class ModifierTransition(
    val key: Key,
    val wasPressed: Boolean,
    val isPressed: Boolean,
)

private fun PointerKeyboardModifiers.transitionsTo(other: PointerKeyboardModifiers): List<ModifierTransition> {
    return listOf(
        ModifierTransition(Key.CtrlLeft, isCtrlPressed, other.isCtrlPressed),
        ModifierTransition(Key.MetaLeft, isMetaPressed, other.isMetaPressed),
        ModifierTransition(Key.AltLeft, isAltPressed, other.isAltPressed),
        ModifierTransition(Key.ShiftLeft, isShiftPressed, other.isShiftPressed),
        ModifierTransition(Key.Function, isFunctionPressed, other.isFunctionPressed),
        ModifierTransition(Key.CapsLock, isCapsLockOn, other.isCapsLockOn),
        ModifierTransition(Key.NumLock, isNumLockOn, other.isNumLockOn),
    )
}

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
