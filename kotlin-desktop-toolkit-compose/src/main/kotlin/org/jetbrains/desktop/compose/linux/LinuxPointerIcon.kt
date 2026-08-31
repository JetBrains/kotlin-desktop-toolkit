package org.jetbrains.desktop.compose.linux

import androidx.compose.ui.input.pointer.PointerIcon
import org.jetbrains.desktop.linux.PointerShape

/**
 * A kotlin-desktop-toolkit pointer shape usable with `Modifier.pointerHoverIcon`, for the shapes Compose
 * has no constant for:
 *
 * ```
 * Modifier.pointerHoverIcon(LinuxPointerIcon(PointerShape.Grab))
 * ```
 */
data class LinuxPointerIcon(val shape: PointerShape) : PointerIcon

/**
 * The single owner of one window's pointer shape.
 *
 * Three things want to set it and they have to be arbitrated rather than each writing to the window:
 * Compose asks for a shape through `PlatformContext.setPointerIcon`, the resize border under the pointer
 * overrides it while the pointer is over a window edge, and typing hides it until the pointer moves
 * again. The override wins over Compose, and hiding wins over both.
 */
internal class LinuxPointerIcons(private val setNativeShape: (PointerShape) -> Unit) {
    private var composeShape: PointerShape = PointerShape.Default
    private var overrideShape: PointerShape? = null
    private var isHiddenUntilPointerMoves = false

    fun apply(pointerIcon: PointerIcon) {
        composeShape = pointerIcon.toPointerShape()
        push()
    }

    /** Takes the pointer shape away from Compose, or hands it back when passed `null`. */
    fun override(shape: PointerShape?) {
        if (overrideShape == shape) return
        overrideShape = shape
        push()
    }

    fun setHiddenUntilPointerMoves(hidden: Boolean) {
        if (isHiddenUntilPointerMoves == hidden) return
        isHiddenUntilPointerMoves = hidden
        push()
    }

    private fun push() {
        setNativeShape(
            when {
                isHiddenUntilPointerMoves -> PointerShape.Hidden
                else -> overrideShape ?: composeShape
            },
        )
    }

    /**
     * Compose's four built-in pointer icons are backed by `java.awt.Cursor` on desktop, and the class
     * wrapping them is internal to compose-ui, so they cannot be unwrapped — they are recognised by
     * identity against the constants instead. Anything else has to arrive as a [LinuxPointerIcon].
     */
    private fun PointerIcon.toPointerShape(): PointerShape = when (this) {
        is LinuxPointerIcon -> shape
        PointerIcon.Text -> PointerShape.Text
        PointerIcon.Hand -> PointerShape.Pointer
        PointerIcon.Crosshair -> PointerShape.Crosshair
        else -> PointerShape.Default
    }
}
