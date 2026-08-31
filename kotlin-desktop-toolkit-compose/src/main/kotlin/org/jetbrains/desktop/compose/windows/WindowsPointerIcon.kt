package org.jetbrains.desktop.compose.windows

import androidx.compose.ui.input.pointer.PointerIcon
import org.jetbrains.desktop.win32.Cursor
import org.jetbrains.desktop.win32.CursorIcon
import org.jetbrains.desktop.win32.Window

/**
 * A kotlin-desktop-toolkit cursor usable with `Modifier.pointerHoverIcon`, for the cursors Compose has
 * no constant for:
 *
 * ```
 * Modifier.pointerHoverIcon(WindowsPointerIcon(CursorIcon.SizeWE))
 * ```
 */
data class WindowsPointerIcon(val icon: CursorIcon) : PointerIcon

/**
 * The cursor of one window.
 *
 * Per window, unlike macOS's process-wide cursor: on win32 the cursor belongs to the window it is over,
 * and `Window.setCursor` is what claims it.
 *
 * Hiding, on the other hand, is a process-wide *counter* (`ShowCursor`), so every hide has to be
 * balanced by exactly one show — hence the flag rather than a straight call. Windows never hides the
 * cursor on its own while the user types, so this is what makes a keyboard-driven UI behave like the
 * macOS one.
 */
internal class WindowsPointerIcons(private val nativeWindow: Window) {
    private var currentIcon: CursorIcon = CursorIcon.Arrow
    private var isHiddenUntilPointerMoves = false

    fun apply(pointerIcon: PointerIcon) {
        val icon = pointerIcon.toCursorIcon()
        currentIcon = icon
        if (!isHiddenUntilPointerMoves) {
            nativeWindow.setCursor(icon)
        }
    }

    fun setHiddenUntilPointerMoves(hidden: Boolean) {
        if (hidden == isHiddenUntilPointerMoves) {
            return
        }
        isHiddenUntilPointerMoves = hidden
        if (hidden) {
            Cursor.hide()
        } else {
            Cursor.show()
            // The icon Compose asked for while the cursor was hidden was never handed to the window.
            nativeWindow.setCursor(currentIcon)
        }
    }

    /** Balances an outstanding hide, so a window can be closed without leaking an invisible cursor. */
    fun onWindowClosing() {
        setHiddenUntilPointerMoves(hidden = false)
    }

    /**
     * Compose's four built-in cursors are backed by `java.awt.Cursor` on desktop, and the class wrapping
     * them is internal to compose-ui, so they cannot be unwrapped — they are recognised by identity
     * against the constants instead. Anything else has to arrive as a [WindowsPointerIcon].
     */
    private fun PointerIcon.toCursorIcon(): CursorIcon = when (this) {
        is WindowsPointerIcon -> icon
        PointerIcon.Text -> CursorIcon.IBeam
        PointerIcon.Hand -> CursorIcon.Hand
        PointerIcon.Crosshair -> CursorIcon.Crosshair
        else -> CursorIcon.Arrow
    }
}
