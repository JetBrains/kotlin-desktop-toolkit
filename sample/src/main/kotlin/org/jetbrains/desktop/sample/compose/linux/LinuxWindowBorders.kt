package org.jetbrains.desktop.sample.compose.linux

import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import org.jetbrains.desktop.linux.PointerShape
import org.jetbrains.desktop.linux.WindowResizeEdge

/**
 * The resize border of a client-side-decorated window.
 *
 * A window the compositor does not decorate has to provide its own resize affordance: the compositor will
 * happily start a resize, but only when the application asks it to, and only for a pointer press it can be
 * pointed at. This is the geometry of "close enough to an edge to mean resize" — a strip just inside each
 * edge, widened at the corners so a diagonal resize is reachable.
 *
 * It is deliberately handled by the window rather than offered to the content: a resize border is a
 * property of the window frame, not of what is drawn inside it, and keeping it here means the sample's
 * content stays the same on every platform.
 */
private val BorderThickness = 6.dp

private val CornerSize = 16.dp

/** The edge a press at [offset] would resize, or `null` when the press is not on the border. */
internal fun resizeEdgeAt(offset: DpOffset, size: DpSize): WindowResizeEdge? {
    if (size.width <= CornerSize * 2 || size.height <= CornerSize * 2) {
        // Too small to carve a border out of without swallowing the whole window.
        return null
    }
    val isLeft = offset.x >= 0.dp && offset.x < BorderThickness
    val isRight = offset.x <= size.width && offset.x > size.width - BorderThickness
    val isTop = offset.y >= 0.dp && offset.y < BorderThickness
    val isBottom = offset.y <= size.height && offset.y > size.height - BorderThickness
    if (!isLeft && !isRight && !isTop && !isBottom) {
        return null
    }
    val isNearLeft = offset.x < CornerSize
    val isNearRight = offset.x > size.width - CornerSize
    val isNearTop = offset.y < CornerSize
    val isNearBottom = offset.y > size.height - CornerSize
    return when {
        (isTop || isLeft) && isNearTop && isNearLeft -> WindowResizeEdge.TopLeft
        (isTop || isRight) && isNearTop && isNearRight -> WindowResizeEdge.TopRight
        (isBottom || isLeft) && isNearBottom && isNearLeft -> WindowResizeEdge.BottomLeft
        (isBottom || isRight) && isNearBottom && isNearRight -> WindowResizeEdge.BottomRight
        isTop -> WindowResizeEdge.Top
        isBottom -> WindowResizeEdge.Bottom
        isLeft -> WindowResizeEdge.Left
        isRight -> WindowResizeEdge.Right
        else -> null
    }
}

internal fun WindowResizeEdge.toPointerShape(): PointerShape = when (this) {
    WindowResizeEdge.Top -> PointerShape.NResize
    WindowResizeEdge.Bottom -> PointerShape.SResize
    WindowResizeEdge.Left -> PointerShape.WResize
    WindowResizeEdge.Right -> PointerShape.EResize
    WindowResizeEdge.TopLeft -> PointerShape.NwResize
    WindowResizeEdge.TopRight -> PointerShape.NeResize
    WindowResizeEdge.BottomLeft -> PointerShape.SwResize
    WindowResizeEdge.BottomRight -> PointerShape.SeResize
}
