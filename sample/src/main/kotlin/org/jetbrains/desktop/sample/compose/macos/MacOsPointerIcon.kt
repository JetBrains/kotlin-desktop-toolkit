package org.jetbrains.desktop.sample.compose.macos

import androidx.compose.ui.input.pointer.PointerIcon
import org.jetbrains.desktop.macos.Cursor

/**
 * A kotlin-desktop-toolkit cursor usable with `Modifier.pointerHoverIcon`, for the cursors Compose has
 * no constant for:
 *
 * ```
 * Modifier.pointerHoverIcon(MacOsPointerIcon(Cursor.Icon.ResizeLeftRightCursor))
 * ```
 */
data class MacOsPointerIcon(val icon: Cursor.Icon) : PointerIcon

/**
 * Applies the cursor Compose asked for.
 *
 * Compose's four built-in cursors are backed by `java.awt.Cursor` on desktop, and the class wrapping
 * them is internal to compose-ui, so they cannot be unwrapped — they are recognised by identity against
 * the constants instead. Anything else has to arrive as a [MacOsPointerIcon].
 */
internal object MacOsPointerIcons {
    fun apply(pointerIcon: PointerIcon) {
        Cursor.icon = pointerIcon.toCursorIcon()
    }

    private fun PointerIcon.toCursorIcon(): Cursor.Icon = when (this) {
        is MacOsPointerIcon -> icon
        PointerIcon.Text -> Cursor.Icon.IBeamCursor
        PointerIcon.Hand -> Cursor.Icon.PointingHandCursor
        PointerIcon.Crosshair -> Cursor.Icon.CrosshairCursor
        else -> Cursor.Icon.ArrowCursor
    }
}
