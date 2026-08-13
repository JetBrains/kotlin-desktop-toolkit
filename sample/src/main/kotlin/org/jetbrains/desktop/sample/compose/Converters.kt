package org.jetbrains.desktop.sample.compose

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerButton
import androidx.compose.ui.input.pointer.PointerButtons
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.KeyModifiersSet
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.MouseButton
import kotlin.math.roundToInt

/**
 * kotlin-desktop-toolkit reports geometry in logical points, while a `ComposeScene` works in pixels and
 * converts to dp itself using the density it was given. One logical point equals one dp, so the scale
 * factor is both the pixel multiplier and the Compose density.
 */
internal fun LogicalSize.toIntSize(scale: Double): IntSize {
    return IntSize((width * scale).roundToInt(), (height * scale).roundToInt())
}

internal fun LogicalSize.toDpSize(): DpSize {
    return DpSize(width.dp, height.dp)
}

internal fun LogicalPoint.toOffset(scale: Double): Offset {
    val physical = toPhysical(scale)
    return Offset(physical.x.toFloat(), physical.y.toFloat())
}

internal fun MouseButton.toComposePointerButton(): PointerButton {
    return when (this) {
        MouseButton.LEFT -> PointerButton.Primary
        MouseButton.RIGHT -> PointerButton.Secondary
        MouseButton.MIDDLE -> PointerButton.Tertiary
        MouseButton.BACK -> PointerButton.Back
        MouseButton.FORWARD -> PointerButton.Forward
        else -> PointerButton(value)
    }
}

internal fun KeyModifiersSet.toPointerKeyboardModifiers(): PointerKeyboardModifiers {
    return PointerKeyboardModifiers(
        isCtrlPressed = control,
        isMetaPressed = command,
        isAltPressed = option,
        isShiftPressed = shift,
        isCapsLockOn = capsLock,
        isFunctionPressed = function,
    )
}

/**
 * The set of buttons currently held down. Compose tracks this itself for drags, so it has to be
 * reported on every pointer event rather than only on press and release.
 */
internal fun currentPointerButtons(): PointerButtons {
    val pressed = Event.pressedMouseButtons()
    return PointerButtons(
        isPrimaryPressed = pressed.contains(MouseButton.LEFT),
        isSecondaryPressed = pressed.contains(MouseButton.RIGHT),
        isTertiaryPressed = pressed.contains(MouseButton.MIDDLE),
        isBackPressed = pressed.contains(MouseButton.BACK),
        isForwardPressed = pressed.contains(MouseButton.FORWARD),
    )
}
