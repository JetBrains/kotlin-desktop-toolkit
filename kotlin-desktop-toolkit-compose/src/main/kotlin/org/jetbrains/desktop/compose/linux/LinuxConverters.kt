package org.jetbrains.desktop.compose.linux

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.pointer.PointerButton
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpRect
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.height
import androidx.compose.ui.unit.width
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPixelsInt
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalRect
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.MouseButton
import kotlin.math.roundToInt

/**
 * kotlin-desktop-toolkit reports Wayland geometry in logical pixels, while a `ComposeScene` works in
 * physical pixels and converts to dp itself using the density it was given. One logical pixel equals one
 * dp, so the surface's scale is both the pixel multiplier and the Compose density.
 *
 * Unlike macOS, the Linux backend has two logical-pixel types: [LogicalPixels] for positions the
 * compositor reports with sub-pixel precision, and [LogicalPixelsInt] for sizes it insists on rounding.
 */
internal fun LogicalPixels.toDp(): Dp = rawLogical.toFloat().dp

internal fun LogicalPixelsInt.toDp(): Dp = rawLogical.toFloat().dp

internal fun LogicalSize.toDpSize(): DpSize = DpSize(width.toDp(), height.toDp())

internal fun LogicalPoint.toDpOffset(): DpOffset = DpOffset(x.toDp(), y.toDp())

internal fun Dp.roundToLogicalPixelsInt(): LogicalPixelsInt = LogicalPixelsInt(value.roundToInt())

internal fun DpSize.roundToLogicalSize(): LogicalSize = LogicalSize(width.roundToLogicalPixelsInt(), height.roundToLogicalPixelsInt())

/**
 * [LogicalSize] used for a window or a drag icon has to have both dimensions strictly positive, which a
 * size Compose measured as zero — an empty drag decoration, say — would violate.
 */
internal fun Size.roundToNonEmptyLogicalSize(density: Density): LogicalSize = with(density) {
    LogicalSize(
        LogicalPixelsInt(width.toDp().value.roundToInt().coerceAtLeast(1)),
        LogicalPixelsInt(height.toDp().value.roundToInt().coerceAtLeast(1)),
    )
}

internal fun DpRect.roundToLogicalRect(): LogicalRect = LogicalRect(
    x = left.roundToLogicalPixelsInt(),
    y = top.roundToLogicalPixelsInt(),
    width = width.roundToLogicalPixelsInt(),
    height = height.roundToLogicalPixelsInt(),
)

internal fun DpOffset.toPxOffset(density: Density): Offset = with(density) {
    Offset(x.toPx(), y.toPx())
}

internal fun LogicalPoint.toPxOffset(density: Density): Offset = toDpOffset().toPxOffset(density)

internal fun DpSize.roundToIntSize(density: Density): IntSize = with(density) {
    IntSize(width.roundToPx(), height.roundToPx())
}

/**
 * Wayland reports buttons with `linux/input-event-codes.h` values, and a mouse may report the fourth and
 * fifth buttons either as SIDE/EXTRA or as BACK/FORWARD depending on its firmware; both mean "back" and
 * "forward" to the user.
 */
internal fun MouseButton.toComposePointerButton(): PointerButton = when (this) {
    MouseButton.LEFT -> PointerButton.Primary
    MouseButton.RIGHT -> PointerButton.Secondary
    MouseButton.MIDDLE -> PointerButton.Tertiary
    MouseButton.SIDE, MouseButton.BACK -> PointerButton.Back
    MouseButton.EXTRA, MouseButton.FORWARD -> PointerButton.Forward
    else -> PointerButton(value)
}

/** Buttons are reported to Compose as key events too, so a shortcut bound to a side button is reachable. */
internal fun MouseButton.toComposeKey(): Key = when (this) {
    MouseButton.LEFT -> Key.Button1
    MouseButton.RIGHT -> Key.Button2
    MouseButton.MIDDLE -> Key.Button3
    MouseButton.SIDE -> Key.Button4
    MouseButton.EXTRA -> Key.Button5
    MouseButton.FORWARD -> Key.Button6
    MouseButton.BACK -> Key.Button7
    MouseButton.TASK -> Key.Button8
    else -> Key.Unknown
}
