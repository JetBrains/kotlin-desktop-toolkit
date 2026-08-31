@file:OptIn(InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.macos

import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.input.pointer.isAltPressed
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.isShiftPressed
import org.jetbrains.desktop.macos.Characters
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.KeyModifiersSet
import org.jetbrains.desktop.macos.MouseButton
import org.jetbrains.desktop.macos.SpecialCharacter
import org.jetbrains.desktop.macos.SpecialKey
import org.jetbrains.desktop.macos.TextInputSource

/**
 * Turns macOS key events into Compose ones.
 *
 * `Key` identifies *which key* was pressed, independently of the character it produces — that is what
 * shortcuts are matched against — so the mapping cannot simply use the typed text. It also cannot use
 * the hardware key code, because macOS lets the user remap the keyboard freely, and a Dvorak user's
 * `KeyCode.ANSI_S` is the `O` key.
 *
 * What the mapping uses instead is the character the key produces *with no modifiers applied*, obtained
 * from the current event through the active input source. For non-Latin layouts (Russian, Serbian) that
 * character is not a shortcut-usable one, so the ASCII layer under Command is consulted instead — the
 * same trick AppKit applications and gpui use, and the reason macOS shortcuts keep working when you
 * switch to Cyrillic.
 *
 * Because it reads "the current event", every function here is only valid while handling a key event.
 */
internal fun Event.KeyDown.toComposeKeyEvent(): KeyEvent {
    return KeyEvent(
        key = keyData().toComposeKey(),
        type = KeyEventType.KeyDown,
        codePoint = characters.toUtf16CodePoint(),
        isCtrlPressed = modifiers.control,
        isMetaPressed = modifiers.command,
        isAltPressed = modifiers.option,
        isShiftPressed = modifiers.shift,
        nativeEvent = this,
    )
}

internal fun Event.KeyUp.toComposeKeyEvent(): KeyEvent {
    return KeyEvent(
        key = keyData().toComposeKey(),
        type = KeyEventType.KeyUp,
        codePoint = characters.toUtf16CodePoint(),
        isCtrlPressed = modifiers.control,
        isMetaPressed = modifiers.command,
        isAltPressed = modifiers.option,
        isShiftPressed = modifiers.shift,
        nativeEvent = this,
    )
}

/**
 * Mouse buttons are reported to Compose as key events too, so that a back/forward button or a shortcut
 * bound to a side button reaches `onKeyEvent` handlers.
 */
internal fun Event.MouseDown.toComposeKeyEvent(keyboardModifiers: PointerKeyboardModifiers): KeyEvent {
    return button.toComposeKeyEvent(this, KeyEventType.KeyDown, keyboardModifiers)
}

internal fun Event.MouseUp.toComposeKeyEvent(keyboardModifiers: PointerKeyboardModifiers): KeyEvent {
    return button.toComposeKeyEvent(this, KeyEventType.KeyUp, keyboardModifiers)
}

internal fun MouseButton.toComposeKeyEvent(nativeEvent: Any, type: KeyEventType, keyboardModifiers: PointerKeyboardModifiers): KeyEvent {
    return KeyEvent(
        key = toComposeKey(),
        type = type,
        codePoint = 0,
        isCtrlPressed = keyboardModifiers.isCtrlPressed,
        isMetaPressed = keyboardModifiers.isMetaPressed,
        isAltPressed = keyboardModifiers.isAltPressed,
        isShiftPressed = keyboardModifiers.isShiftPressed,
        nativeEvent = nativeEvent,
    )
}

// KDT reports MouseButton.value 0-based, so button 0 is Compose's Button1.
private fun MouseButton.toComposeKey(): Key = when (value) {
    0 -> Key.Button1
    1 -> Key.Button2
    2 -> Key.Button3
    3 -> Key.Button4
    4 -> Key.Button5
    5 -> Key.Button6
    6 -> Key.Button7
    7 -> Key.Button8
    else -> Key.Unknown
}

private class KeyData(
    val charactersIgnoringModifiers: Characters,
    val modifiers: KeyModifiersSet,
)

private fun Event.KeyDown.keyData() = KeyData(charactersIgnoringModifiers, modifiers)

private fun Event.KeyUp.keyData() = KeyData(charactersIgnoringModifiers, modifiers)

private fun KeyData.toComposeKey(): Key {
    // Arrows, function keys, Home/End and friends arrive as codepoints from AppKit's private-use range;
    // they are layout-independent, so they are resolved before any character-based lookup.
    charactersIgnoringModifiers.specialKey?.let { specialKey ->
        keyFromSpecialKey(specialKey)?.let { return it }
    }
    // Unlike `charactersIgnoringModifiers`, this really does drop every modifier: Shift+Tab reports the
    // back-tab character in the former but a plain tab here.
    val keyWithNoModifiers = Event.charactersByApplyingModifiersForCurrentEvent(KeyModifiersSet.create())
    keyWithNoModifiers.specialCharacter?.let { specialCharacter ->
        keyFromSpecialCharacter(specialCharacter)?.let { return it }
    }
    val isAsciiInputSource = TextInputSource.current()?.let { TextInputSource.isAsciiCapable(it) } ?: true
    val text = if (isAsciiInputSource) {
        keyWithNoModifiers.text
    } else {
        // Non-ASCII layouts keep an ASCII layer under Command, which is what shortcuts are bound to.
        Event.charactersByApplyingModifiersForCurrentEvent(KeyModifiersSet.create(command = true)).text
    }
    return keyFromAsciiSymbols(text, modifiers.numericPad) ?: Key.Unknown
}

private fun keyFromSpecialCharacter(specialCharacter: SpecialCharacter): Key? = when (specialCharacter) {
    SpecialCharacter.EnterCharacter -> Key.Enter
    SpecialCharacter.CarriageReturnCharacter -> Key.Enter
    SpecialCharacter.NewlineCharacter -> Key.Enter
    SpecialCharacter.BackspaceCharacter -> Key.Backspace
    SpecialCharacter.TabCharacter -> Key.Tab
    SpecialCharacter.BackTabCharacter -> Key.Tab
    SpecialCharacter.DeleteCharacter -> Key.Delete
    SpecialCharacter.EscapeCharacter -> Key.Escape
    SpecialCharacter.SpaceCharacter -> Key.Spacebar
    else -> null
}

private fun keyFromSpecialKey(specialKey: SpecialKey): Key? = when (specialKey) {
    SpecialKey.UpArrowFunctionKey -> Key.DirectionUp
    SpecialKey.DownArrowFunctionKey -> Key.DirectionDown
    SpecialKey.LeftArrowFunctionKey -> Key.DirectionLeft
    SpecialKey.RightArrowFunctionKey -> Key.DirectionRight

    SpecialKey.F1FunctionKey -> Key.F1
    SpecialKey.F2FunctionKey -> Key.F2
    SpecialKey.F3FunctionKey -> Key.F3
    SpecialKey.F4FunctionKey -> Key.F4
    SpecialKey.F5FunctionKey -> Key.F5
    SpecialKey.F6FunctionKey -> Key.F6
    SpecialKey.F7FunctionKey -> Key.F7
    SpecialKey.F8FunctionKey -> Key.F8
    SpecialKey.F9FunctionKey -> Key.F9
    SpecialKey.F10FunctionKey -> Key.F10
    SpecialKey.F11FunctionKey -> Key.F11
    SpecialKey.F12FunctionKey -> Key.F12
    // F13 and up have no androidx `Key` constant, so they stay Unknown.

    SpecialKey.InsertFunctionKey -> Key.Insert
    SpecialKey.DeleteFunctionKey -> Key.Delete
    SpecialKey.HomeFunctionKey -> Key.MoveHome
    SpecialKey.EndFunctionKey -> Key.MoveEnd
    SpecialKey.PageUpFunctionKey -> Key.PageUp
    SpecialKey.PageDownFunctionKey -> Key.PageDown

    SpecialKey.PrintScreenFunctionKey -> Key.PrintScreen
    SpecialKey.ScrollLockFunctionKey -> Key.ScrollLock
    SpecialKey.PauseFunctionKey -> Key.Break
    SpecialKey.MenuFunctionKey -> Key.Menu
    SpecialKey.ClearLineFunctionKey -> Key.NumLock

    SpecialKey.NextFunctionKey -> Key.Forward
    SpecialKey.PrevFunctionKey -> Key.Back
    SpecialKey.HelpFunctionKey -> Key.Insert

    else -> null
}

private fun keyFromAsciiSymbols(text: String, isNumericPad: Boolean): Key? {
    return if (isNumericPad) {
        keyFromNumericPadSymbols(text) ?: keyFromMainBlockSymbols(text)
    } else {
        keyFromLetters(text) ?: keyFromMainBlockSymbols(text)
    }
}

private fun keyFromLetters(text: String): Key? = when (text) {
    "a" -> Key.A
    "b" -> Key.B
    "c" -> Key.C
    "d" -> Key.D
    "e" -> Key.E
    "f" -> Key.F
    "g" -> Key.G
    "h" -> Key.H
    "i" -> Key.I
    "j" -> Key.J
    "k" -> Key.K
    "l" -> Key.L
    "m" -> Key.M
    "n" -> Key.N
    "o" -> Key.O
    "p" -> Key.P
    "q" -> Key.Q
    "r" -> Key.R
    "s" -> Key.S
    "t" -> Key.T
    "u" -> Key.U
    "v" -> Key.V
    "w" -> Key.W
    "x" -> Key.X
    "y" -> Key.Y
    "z" -> Key.Z
    else -> null
}

/** Symbols on the base layer of the US layout's main block. */
private fun keyFromMainBlockSymbols(text: String): Key? = when (text) {
    "0" -> Key.Zero
    "1" -> Key.One
    "2" -> Key.Two
    "3" -> Key.Three
    "4" -> Key.Four
    "5" -> Key.Five
    "6" -> Key.Six
    "7" -> Key.Seven
    "8" -> Key.Eight
    "9" -> Key.Nine

    "=" -> Key.Equals
    "-" -> Key.Minus
    "+" -> Key.Plus
    "*" -> Key.Multiply
    "]" -> Key.RightBracket
    "[" -> Key.LeftBracket
    "'" -> Key.Apostrophe
    ";" -> Key.Semicolon
    "\\" -> Key.Backslash
    "," -> Key.Comma
    "/" -> Key.Slash
    "." -> Key.Period
    "`" -> Key.Grave
    "@" -> Key.At

    "\n", "\r", "\r\n" -> Key.Enter
    "\t" -> Key.Tab
    " " -> Key.Spacebar
    "\u0008" -> Key.Backspace
    "\u001B" -> Key.Escape
    "\u21EA" -> Key.CapsLock

    else -> null
}

private fun keyFromNumericPadSymbols(text: String): Key? = when (text) {
    "0" -> Key.NumPad0
    "1" -> Key.NumPad1
    "2" -> Key.NumPad2
    "3" -> Key.NumPad3
    "4" -> Key.NumPad4
    "5" -> Key.NumPad5
    "6" -> Key.NumPad6
    "7" -> Key.NumPad7
    "8" -> Key.NumPad8
    "9" -> Key.NumPad9

    "," -> Key.NumPadComma
    "." -> Key.NumPadDot
    "*" -> Key.NumPadMultiply
    "+" -> Key.NumPadAdd
    "/" -> Key.NumPadDivide
    "-" -> Key.NumPadSubtract
    "=" -> Key.NumPadEquals
    "(" -> Key.NumPadLeftParenthesis
    ")" -> Key.NumPadRightParenthesis
    "\n", "\r", "\r\n" -> Key.NumPadEnter

    else -> null
}

/** The codepoint the key produced, or 0 when it produced no text of its own. */
private fun Characters.toUtf16CodePoint(): Int = when {
    // Characters from the Unicode private use area encode arrows and function keys, not text.
    specialKey != null -> 0
    text.isEmpty() -> 0
    else -> text.codePointAt(0)
}
