package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h

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
