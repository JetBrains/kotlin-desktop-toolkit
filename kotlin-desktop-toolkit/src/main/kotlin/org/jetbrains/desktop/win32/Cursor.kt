package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h

/**
 * Controls the visibility of the system mouse cursor.
 *
 * Wraps Win32 [`ShowCursor`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showcursor),
 * which keeps a single process-wide display counter rather than a boolean: [hide] decrements it and
 * [show] increments it, and the cursor is drawn only while the counter is `>= 0`. The counter therefore
 * behaves like a stack of outstanding hide requests, so every [hide] must be paired with a matching
 * [show] to restore the previous state. Its initial value is `0` when a mouse is installed and `-1` otherwise.
 */
public object Cursor {
    /**
     * Increments the display counter, undoing one prior [hide]. The cursor becomes visible once the
     * counter reaches `0`.
     *
     * @return the new display counter value.
     */
    public fun show(): Int {
        return ffiDownCall {
            desktop_win32_h.cursor_show()
        }
    }

    /**
     * Decrements the display counter, hiding the cursor while it stays below `0`. Pair each call with
     * a later [show].
     *
     * @return the new display counter value.
     */
    public fun hide(): Int {
        return ffiDownCall {
            desktop_win32_h.cursor_hide()
        }
    }
}

public enum class CursorIcon {
    Arrow,
    IBeam,
    Wait,
    Crosshair,
    UpArrow,

    SizeNWSE,
    SizeNESW,
    SizeWE,
    SizeNS,

    SizeAll,
    NotAllowed,
    Hand,
    AppStarting,
    Help,
    Pin,
    Person,
    ;

    internal fun toNative(): Int = when (this) {
        Arrow -> desktop_win32_h.NativeCursorIcon_Arrow()
        IBeam -> desktop_win32_h.NativeCursorIcon_IBeam()
        Wait -> desktop_win32_h.NativeCursorIcon_Wait()
        Crosshair -> desktop_win32_h.NativeCursorIcon_Crosshair()
        UpArrow -> desktop_win32_h.NativeCursorIcon_UpArrow()
        SizeNWSE -> desktop_win32_h.NativeCursorIcon_SizeNWSE()
        SizeNESW -> desktop_win32_h.NativeCursorIcon_SizeNESW()
        SizeWE -> desktop_win32_h.NativeCursorIcon_SizeWE()
        SizeNS -> desktop_win32_h.NativeCursorIcon_SizeNS()
        SizeAll -> desktop_win32_h.NativeCursorIcon_SizeAll()
        NotAllowed -> desktop_win32_h.NativeCursorIcon_NotAllowed()
        Hand -> desktop_win32_h.NativeCursorIcon_Hand()
        AppStarting -> desktop_win32_h.NativeCursorIcon_AppStarting()
        Help -> desktop_win32_h.NativeCursorIcon_Help()
        Pin -> desktop_win32_h.NativeCursorIcon_Pin()
        Person -> desktop_win32_h.NativeCursorIcon_Person()
    }
}
