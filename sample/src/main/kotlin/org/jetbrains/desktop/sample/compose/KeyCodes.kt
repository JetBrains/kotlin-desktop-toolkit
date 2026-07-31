package org.jetbrains.desktop.sample.compose

import androidx.compose.ui.input.key.Key
import org.jetbrains.desktop.macos.KeyCode

/**
 * Maps macOS virtual key codes reported by kotlin-desktop-toolkit onto Compose's [Key].
 *
 * These are physical key positions, not characters — the typed text arrives separately as
 * `Event.KeyDown.keyWithModifiers`.
 */
internal fun KeyCode.toComposeKey(): Key {
    return when (this) {
        // Letters
        KeyCode.ANSI_A -> Key.A
        KeyCode.ANSI_B -> Key.B
        KeyCode.ANSI_C -> Key.C
        KeyCode.ANSI_D -> Key.D
        KeyCode.ANSI_E -> Key.E
        KeyCode.ANSI_F -> Key.F
        KeyCode.ANSI_G -> Key.G
        KeyCode.ANSI_H -> Key.H
        KeyCode.ANSI_I -> Key.I
        KeyCode.ANSI_J -> Key.J
        KeyCode.ANSI_K -> Key.K
        KeyCode.ANSI_L -> Key.L
        KeyCode.ANSI_M -> Key.M
        KeyCode.ANSI_N -> Key.N
        KeyCode.ANSI_O -> Key.O
        KeyCode.ANSI_P -> Key.P
        KeyCode.ANSI_Q -> Key.Q
        KeyCode.ANSI_R -> Key.R
        KeyCode.ANSI_S -> Key.S
        KeyCode.ANSI_T -> Key.T
        KeyCode.ANSI_U -> Key.U
        KeyCode.ANSI_V -> Key.V
        KeyCode.ANSI_W -> Key.W
        KeyCode.ANSI_X -> Key.X
        KeyCode.ANSI_Y -> Key.Y
        KeyCode.ANSI_Z -> Key.Z

        // Numbers
        KeyCode.ANSI_0 -> Key.Zero
        KeyCode.ANSI_1 -> Key.One
        KeyCode.ANSI_2 -> Key.Two
        KeyCode.ANSI_3 -> Key.Three
        KeyCode.ANSI_4 -> Key.Four
        KeyCode.ANSI_5 -> Key.Five
        KeyCode.ANSI_6 -> Key.Six
        KeyCode.ANSI_7 -> Key.Seven
        KeyCode.ANSI_8 -> Key.Eight
        KeyCode.ANSI_9 -> Key.Nine

        // Punctuation
        KeyCode.ANSI_Equal -> Key.Equals
        KeyCode.ANSI_Minus -> Key.Minus
        KeyCode.ANSI_RightBracket -> Key.RightBracket
        KeyCode.ANSI_LeftBracket -> Key.LeftBracket
        KeyCode.ANSI_Quote -> Key.Apostrophe
        KeyCode.ANSI_Semicolon -> Key.Semicolon
        KeyCode.ANSI_Backslash -> Key.Backslash
        KeyCode.ANSI_Comma -> Key.Comma
        KeyCode.ANSI_Slash -> Key.Slash
        KeyCode.ANSI_Period -> Key.Period
        KeyCode.ANSI_Grave -> Key.Grave

        // Keypad
        KeyCode.ANSI_Keypad0 -> Key.NumPad0
        KeyCode.ANSI_Keypad1 -> Key.NumPad1
        KeyCode.ANSI_Keypad2 -> Key.NumPad2
        KeyCode.ANSI_Keypad3 -> Key.NumPad3
        KeyCode.ANSI_Keypad4 -> Key.NumPad4
        KeyCode.ANSI_Keypad5 -> Key.NumPad5
        KeyCode.ANSI_Keypad6 -> Key.NumPad6
        KeyCode.ANSI_Keypad7 -> Key.NumPad7
        KeyCode.ANSI_Keypad8 -> Key.NumPad8
        KeyCode.ANSI_Keypad9 -> Key.NumPad9
        KeyCode.ANSI_KeypadDecimal -> Key.NumPadDot
        KeyCode.ANSI_KeypadMultiply -> Key.NumPadMultiply
        KeyCode.ANSI_KeypadPlus -> Key.NumPadAdd
        KeyCode.ANSI_KeypadDivide -> Key.NumPadDivide
        KeyCode.ANSI_KeypadEnter -> Key.NumPadEnter
        KeyCode.ANSI_KeypadMinus -> Key.NumPadSubtract
        KeyCode.ANSI_KeypadEquals -> Key.NumPadEquals

        // Editing
        KeyCode.Return -> Key.Enter
        KeyCode.Tab -> Key.Tab
        KeyCode.Space -> Key.Spacebar
        KeyCode.Delete -> Key.Backspace
        KeyCode.Escape -> Key.Escape
        KeyCode.ForwardDelete -> Key.Delete

        // Modifiers
        KeyCode.Command -> Key.MetaLeft
        KeyCode.RightCommand -> Key.MetaRight
        KeyCode.Shift -> Key.ShiftLeft
        KeyCode.RightShift -> Key.ShiftRight
        KeyCode.CapsLock -> Key.CapsLock
        KeyCode.Option -> Key.AltLeft
        KeyCode.RightOption -> Key.AltRight
        KeyCode.Control -> Key.CtrlLeft
        KeyCode.RightControl -> Key.CtrlRight

        // Function keys
        KeyCode.F1 -> Key.F1
        KeyCode.F2 -> Key.F2
        KeyCode.F3 -> Key.F3
        KeyCode.F4 -> Key.F4
        KeyCode.F5 -> Key.F5
        KeyCode.F6 -> Key.F6
        KeyCode.F7 -> Key.F7
        KeyCode.F8 -> Key.F8
        KeyCode.F9 -> Key.F9
        KeyCode.F10 -> Key.F10
        KeyCode.F11 -> Key.F11
        KeyCode.F12 -> Key.F12

        // Navigation
        KeyCode.Help -> Key.Help
        KeyCode.Home -> Key.MoveHome
        KeyCode.PageUp -> Key.PageUp
        KeyCode.End -> Key.MoveEnd
        KeyCode.PageDown -> Key.PageDown
        KeyCode.LeftArrow -> Key.DirectionLeft
        KeyCode.RightArrow -> Key.DirectionRight
        KeyCode.DownArrow -> Key.DirectionDown
        KeyCode.UpArrow -> Key.DirectionUp

        else -> Key.Unknown
    }
}
