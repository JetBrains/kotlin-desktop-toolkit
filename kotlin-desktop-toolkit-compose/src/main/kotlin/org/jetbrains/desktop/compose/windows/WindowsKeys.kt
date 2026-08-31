package org.jetbrains.desktop.compose.windows

import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import org.jetbrains.desktop.win32.Keyboard
import org.jetbrains.desktop.win32.VirtualKey
import org.jetbrains.desktop.win32.PointerButton as Win32PointerButton

/**
 * The virtual key a `WM_KEYDOWN` carries, as the Compose [Key] for it.
 *
 * Virtual keys are layout-dependent for anything but the fixed keys: the OEM keys below name positions
 * on a US layout, which is the mapping every Windows application ends up with — the *character* the key
 * produces comes from `ToUnicode`, not from here (see [org.jetbrains.desktop.win32.Event.KeyDown.toUnicode]).
 *
 * Left and right modifiers both map onto the left constant: Compose's shortcut matching keys off the
 * generic modifier flags, and a `WM_KEYDOWN` for the bare `VK_SHIFT` cannot tell the sides apart anyway.
 */
internal fun VirtualKey.toComposeKey(): Key = when (this) {
    // digits
    VirtualKey.Number0 -> Key.Zero
    VirtualKey.Number1 -> Key.One
    VirtualKey.Number2 -> Key.Two
    VirtualKey.Number3 -> Key.Three
    VirtualKey.Number4 -> Key.Four
    VirtualKey.Number5 -> Key.Five
    VirtualKey.Number6 -> Key.Six
    VirtualKey.Number7 -> Key.Seven
    VirtualKey.Number8 -> Key.Eight
    VirtualKey.Number9 -> Key.Nine

    // letters
    VirtualKey.A -> Key.A
    VirtualKey.B -> Key.B
    VirtualKey.C -> Key.C
    VirtualKey.D -> Key.D
    VirtualKey.E -> Key.E
    VirtualKey.F -> Key.F
    VirtualKey.G -> Key.G
    VirtualKey.H -> Key.H
    VirtualKey.I -> Key.I
    VirtualKey.J -> Key.J
    VirtualKey.K -> Key.K
    VirtualKey.L -> Key.L
    VirtualKey.M -> Key.M
    VirtualKey.N -> Key.N
    VirtualKey.O -> Key.O
    VirtualKey.P -> Key.P
    VirtualKey.Q -> Key.Q
    VirtualKey.R -> Key.R
    VirtualKey.S -> Key.S
    VirtualKey.T -> Key.T
    VirtualKey.U -> Key.U
    VirtualKey.V -> Key.V
    VirtualKey.W -> Key.W
    VirtualKey.X -> Key.X
    VirtualKey.Y -> Key.Y
    VirtualKey.Z -> Key.Z

    // punctuation, named after the US layout positions
    VirtualKey.OemPlus -> Key.Equals
    VirtualKey.OemMinus -> Key.Minus
    VirtualKey.OemComma -> Key.Comma
    VirtualKey.OemPeriod -> Key.Period
    VirtualKey.Oem1 -> Key.Semicolon
    VirtualKey.Oem2 -> Key.Slash
    VirtualKey.Oem3 -> Key.Grave
    VirtualKey.Oem4 -> Key.LeftBracket
    VirtualKey.Oem5 -> Key.Backslash
    VirtualKey.Oem6 -> Key.RightBracket
    VirtualKey.Oem7 -> Key.Apostrophe

    // numpad
    VirtualKey.NumberPad0 -> Key.NumPad0
    VirtualKey.NumberPad1 -> Key.NumPad1
    VirtualKey.NumberPad2 -> Key.NumPad2
    VirtualKey.NumberPad3 -> Key.NumPad3
    VirtualKey.NumberPad4 -> Key.NumPad4
    VirtualKey.NumberPad5 -> Key.NumPad5
    VirtualKey.NumberPad6 -> Key.NumPad6
    VirtualKey.NumberPad7 -> Key.NumPad7
    VirtualKey.NumberPad8 -> Key.NumPad8
    VirtualKey.NumberPad9 -> Key.NumPad9
    VirtualKey.Decimal -> Key.NumPadDot
    VirtualKey.Separator -> Key.NumPadComma
    VirtualKey.Add -> Key.NumPadAdd
    VirtualKey.Subtract -> Key.NumPadSubtract
    VirtualKey.Multiply -> Key.NumPadMultiply
    VirtualKey.Divide -> Key.NumPadDivide
    VirtualKey.NumberKeyLock -> Key.NumLock

    // modifiers
    VirtualKey.Shift, VirtualKey.LeftShift, VirtualKey.RightShift -> Key.ShiftLeft
    VirtualKey.Control, VirtualKey.LeftControl, VirtualKey.RightControl -> Key.CtrlLeft
    VirtualKey.Menu, VirtualKey.LeftMenu, VirtualKey.RightMenu -> Key.AltLeft
    VirtualKey.LeftWindows, VirtualKey.RightWindows -> Key.MetaLeft

    // editing and navigation
    VirtualKey.Enter -> Key.Enter
    VirtualKey.Tab -> Key.Tab
    VirtualKey.Space -> Key.Spacebar
    VirtualKey.Back -> Key.Backspace
    VirtualKey.Escape -> Key.Escape
    VirtualKey.CapitalLock -> Key.CapsLock
    VirtualKey.Snapshot -> Key.PrintScreen
    VirtualKey.Scroll -> Key.ScrollLock
    VirtualKey.Pause -> Key.Break
    VirtualKey.Insert -> Key.Insert
    VirtualKey.Delete -> Key.Delete
    VirtualKey.Home -> Key.MoveHome
    VirtualKey.End -> Key.MoveEnd
    VirtualKey.PageUp -> Key.PageUp
    VirtualKey.PageDown -> Key.PageDown
    VirtualKey.Left -> Key.DirectionLeft
    VirtualKey.Right -> Key.DirectionRight
    VirtualKey.Up -> Key.DirectionUp
    VirtualKey.Down -> Key.DirectionDown
    VirtualKey.Application -> Key.Menu

    // function keys
    VirtualKey.F1 -> Key.F1
    VirtualKey.F2 -> Key.F2
    VirtualKey.F3 -> Key.F3
    VirtualKey.F4 -> Key.F4
    VirtualKey.F5 -> Key.F5
    VirtualKey.F6 -> Key.F6
    VirtualKey.F7 -> Key.F7
    VirtualKey.F8 -> Key.F8
    VirtualKey.F9 -> Key.F9
    VirtualKey.F10 -> Key.F10
    VirtualKey.F11 -> Key.F11
    VirtualKey.F12 -> Key.F12

    // media and browser keys
    VirtualKey.VolumeUp -> Key.VolumeUp
    VirtualKey.VolumeDown -> Key.VolumeDown
    VirtualKey.VolumeMute -> Key.VolumeMute
    VirtualKey.MediaNextTrack -> Key.MediaNext
    VirtualKey.MediaPreviousTrack -> Key.MediaPrevious
    VirtualKey.MediaStop -> Key.MediaStop
    VirtualKey.MediaPlayPause -> Key.MediaPlayPause
    VirtualKey.GoBack -> Key.NavigateOut
    VirtualKey.GoForward -> Key.NavigateIn

    else -> Key.Unknown
}

/**
 * The modifiers held right now, read straight from the keyboard.
 *
 * Windows sends no equivalent of AppKit's `ModifiersChanged` — a modifier press is an ordinary
 * `WM_KEYDOWN` and nothing reports the resulting mask — so every event that needs modifiers asks
 * `GetKeyboardState` for them instead of tracking transitions.
 */
internal fun currentKeyboardModifiers(): PointerKeyboardModifiers {
    val keyboard = Keyboard.getState()
    return PointerKeyboardModifiers(
        isCtrlPressed = keyboard.isKeyDown(VirtualKey.Control),
        isMetaPressed = keyboard.isKeyDown(VirtualKey.LeftWindows) || keyboard.isKeyDown(VirtualKey.RightWindows),
        isAltPressed = keyboard.isKeyDown(VirtualKey.Menu),
        isShiftPressed = keyboard.isKeyDown(VirtualKey.Shift),
        isCapsLockOn = keyboard.isKeyToggled(VirtualKey.CapitalLock),
        isNumLockOn = keyboard.isKeyToggled(VirtualKey.NumberKeyLock),
        isScrollLockOn = keyboard.isKeyToggled(VirtualKey.Scroll),
    )
}

internal fun VirtualKey.isModifier(): Boolean = when (this) {
    VirtualKey.Shift, VirtualKey.LeftShift, VirtualKey.RightShift,
    VirtualKey.Control, VirtualKey.LeftControl, VirtualKey.RightControl,
    VirtualKey.Menu, VirtualKey.LeftMenu, VirtualKey.RightMenu,
    VirtualKey.LeftWindows, VirtualKey.RightWindows,
    -> true
    else -> false
}

/** Pointer buttons are also offered as key events, so a shortcut bound to a side button is reachable. */
internal fun Win32PointerButton.toComposeKey(): Key = when (this) {
    Win32PointerButton.Left -> Key.Button1
    Win32PointerButton.Right -> Key.Button2
    Win32PointerButton.Middle -> Key.Button3
    Win32PointerButton.XButton1 -> Key.Button4
    Win32PointerButton.XButton2 -> Key.Button5
    else -> Key.Unknown
}
