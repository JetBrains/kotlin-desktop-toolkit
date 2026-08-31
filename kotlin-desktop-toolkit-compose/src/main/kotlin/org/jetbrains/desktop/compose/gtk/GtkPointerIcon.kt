package org.jetbrains.desktop.compose.gtk

import androidx.compose.ui.input.pointer.PointerIcon
import org.jetbrains.desktop.gtk.PointerShape

/**
 * A kotlin-desktop-toolkit pointer shape usable with `Modifier.pointerHoverIcon`, for the shapes Compose
 * has no constant for:
 *
 * ```
 * Modifier.pointerHoverIcon(GtkPointerIcon(PointerShape.Grab))
 * ```
 */
data class GtkPointerIcon(val shape: PointerShape) : PointerIcon

/**
 * The single owner of one window's pointer shape.
 *
 * Two things want to set it and they have to be arbitrated rather than each writing to the window:
 * Compose asks for a shape through `PlatformContext.setPointerIcon`, and typing hides the pointer until it
 * moves again. Hiding wins.
 *
 * Unlike the Wayland backend there is no third claimant: GTK draws and drives the window's resize border
 * itself, so this layer never has to take the shape over for one.
 */
internal class GtkPointerIcons(private val setNativeShape: (PointerShape) -> Unit) {
    private var composeShape: PointerShape = PointerShape.Default
    private var isHiddenUntilPointerMoves = false

    fun apply(pointerIcon: PointerIcon) {
        val shape = pointerIcon.toPointerShape()
        if (composeShape == shape) return
        composeShape = shape
        push()
    }

    fun setHiddenUntilPointerMoves(hidden: Boolean) {
        if (isHiddenUntilPointerMoves == hidden) return
        isHiddenUntilPointerMoves = hidden
        push()
    }

    private fun push() {
        setNativeShape(if (isHiddenUntilPointerMoves) PointerShape.Hidden else composeShape)
    }

    /**
     * Compose's four built-in pointer icons are backed by `java.awt.Cursor` on desktop, and the class
     * wrapping them is internal to compose-ui, so they cannot be unwrapped — they are recognised by
     * identity against the constants instead. Anything else has to arrive as a [GtkPointerIcon].
     */
    private fun PointerIcon.toPointerShape(): PointerShape = when (this) {
        is GtkPointerIcon -> shape
        PointerIcon.Text -> PointerShape.Text
        PointerIcon.Hand -> PointerShape.Pointer
        PointerIcon.Crosshair -> PointerShape.Crosshair
        else -> PointerShape.Default
    }
}
