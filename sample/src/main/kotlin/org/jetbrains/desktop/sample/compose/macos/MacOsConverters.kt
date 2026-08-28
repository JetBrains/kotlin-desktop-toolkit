package org.jetbrains.desktop.sample.compose.macos

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.input.pointer.PointerButton
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpRect
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalRect
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.MouseButton

/**
 * kotlin-desktop-toolkit reports geometry in logical points, while a `ComposeScene` works in pixels and
 * converts to dp itself using the density it was given. One logical point equals one dp, so the window's
 * scale factor is both the pixel multiplier and the Compose density.
 */
internal fun LogicalSize.roundToIntSize(density: Density): IntSize {
    return with(density) {
        IntSize(width.dp.roundToPx(), height.dp.roundToPx())
    }
}

internal fun LogicalSize.toDpSize(): DpSize {
    return DpSize(width.dp, height.dp)
}

internal fun LogicalPoint.toDpOffset(): DpOffset {
    return DpOffset(x.dp, y.dp)
}

internal fun DpOffset.toLogicalPoint(): LogicalPoint {
    return LogicalPoint(x.value.toDouble(), y.value.toDouble())
}

internal fun DpSize.toLogicalSize(): LogicalSize {
    return LogicalSize(width.value.toDouble(), height.value.toDouble())
}

internal fun DpOffset.toPxOffset(density: Density): Offset = with(density) {
    Offset(x.toPx(), y.toPx())
}

internal fun Offset.toLogicalPoint(density: Density): LogicalPoint {
    return with(density) {
        LogicalPoint(x.toDp().value.toDouble(), y.toDp().value.toDouble())
    }
}

internal fun Size.toLogicalSize(density: Density): LogicalSize {
    return with(density) {
        LogicalSize(
            width.toDp().value.toDouble(),
            height.toDp().value.toDouble(),
        )
    }
}

/** Screen coordinates: [contentOrigin] is the window content's top-left in the global desktop space. */
internal fun DpRect.toScreenLogicalRect(contentOrigin: LogicalPoint): LogicalRect {
    return LogicalRect(
        LogicalPoint(
            contentOrigin.x + left.value.toDouble(),
            contentOrigin.y + top.value.toDouble(),
        ),
        LogicalSize(
            (right - left).value.toDouble(),
            (bottom - top).value.toDouble(),
        ),
    )
}

internal operator fun LogicalPoint.plus(other: LogicalPoint): LogicalPoint = LogicalPoint(x + other.x, y + other.y)

internal operator fun LogicalPoint.minus(other: LogicalPoint): LogicalPoint = LogicalPoint(x - other.x, y - other.y)

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
