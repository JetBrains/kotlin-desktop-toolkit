package org.jetbrains.desktop.compose.windows

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.pointer.PointerButton
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.PhysicalPoint
import org.jetbrains.desktop.win32.PhysicalSize
import kotlin.math.roundToInt
import org.jetbrains.desktop.win32.PointerButton as Win32PointerButton

/**
 * kotlin-desktop-toolkit reports win32 geometry in logical pixels, while a `ComposeScene` works in
 * pixels and converts to dp itself using the density it was given. One logical pixel equals one dp, so
 * the window's scale factor is both the pixel multiplier and the Compose density.
 *
 * Unlike the macOS backend, win32 logical geometry is already `Float`-valued, so nothing widens here.
 * Physical geometry — which is what the non-client messages and the screen origin are reported in — has
 * to be divided by the scale first.
 */
internal fun LogicalSize.toDpSize(): DpSize {
    return DpSize(width.dp, height.dp)
}

internal fun LogicalPoint.toDpOffset(): DpOffset {
    return DpOffset(x.dp, y.dp)
}

internal fun DpOffset.toLogicalPoint(): LogicalPoint {
    return LogicalPoint(x.value, y.value)
}

internal fun DpSize.toLogicalSize(): LogicalSize {
    return LogicalSize(width.value, height.value)
}

internal fun DpOffset.toPxOffset(density: Density): Offset = with(density) {
    Offset(x.toPx(), y.toPx())
}

/** Physical pixels, which is what a drag image's cursor hotspot is measured in. */
internal fun Offset.toPhysicalPoint(): PhysicalPoint = PhysicalPoint(x.roundToInt(), y.roundToInt())

internal fun PhysicalPoint.toDpOffset(scale: Float): DpOffset = toLogical(scale).toDpOffset()

internal fun PhysicalSize.toDpSize(scale: Float): DpSize = toLogical(scale).toDpSize()

internal fun Win32PointerButton.toComposePointerButton(): PointerButton = when (this) {
    Win32PointerButton.Left -> PointerButton.Primary
    Win32PointerButton.Right -> PointerButton.Secondary
    Win32PointerButton.Middle -> PointerButton.Tertiary
    Win32PointerButton.XButton1 -> PointerButton.Back
    Win32PointerButton.XButton2 -> PointerButton.Forward
    else -> PointerButton(0)
}
