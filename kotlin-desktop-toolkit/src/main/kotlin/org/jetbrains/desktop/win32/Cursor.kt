package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h

/**
 * Controls visibility of the system mouse cursor via Win32
 * [`ShowCursor`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-showcursor).
 *
 * Visibility is a counter, not a boolean: [hide] decrements it, [show] increments it, and the cursor is
 * drawn while the counter is `>= 0`. It behaves like a stack of hide requests, so each [hide] must be
 * balanced by a [show]. The initial value is `0` with a mouse installed and `-1` otherwise.
 *
 * The counter only affects the cursor while it is over one of the calling application's own windows;
 * elsewhere the system draws the cursor per its own state, so a windowless process can move the counter
 * without changing anything on screen.
 */
public object Cursor {
    /**
     * Increments the display counter, undoing one [hide]; the cursor reappears once it reaches `0`.
     *
     * @return the new counter value.
     */
    public fun show(): Int {
        return ffiDownCall {
            desktop_win32_h.cursor_show()
        }
    }

    /**
     * Decrements the display counter, hiding the cursor while it is below `0`. Balance each call with a [show].
     *
     * @return the new counter value.
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
