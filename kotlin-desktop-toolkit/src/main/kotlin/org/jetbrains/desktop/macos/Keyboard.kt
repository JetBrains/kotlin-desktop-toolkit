package org.jetbrains.desktop.macos

/*  MacOSX15.2
*  Summary:
*    Virtual keycodes
*
*  Discussion:
*    These constants are the virtual keycodes defined originally in
*    Inside Mac Volume V, pg. V-191. They identify physical keys on a
*    keyboard. Those constants with "ANSI" in the name are labeled
*    according to the key position on an ANSI-standard US keyboard.
*    For example, kVK_ANSI_A indicates the virtual keycode for the key
*    with the letter 'A' in the US keyboard layout. Other keyboard
*    layouts may have the 'A' key label on a different physical key;
*    in this case, pressing 'A' will generate a different virtual
*    keycode.
*/
@JvmInline
public value class KeyCode private constructor(internal val value: Short) {
    @Suppress("MemberVisibilityCanBePrivate")
    public companion object {
        public val ANSI_A: KeyCode = KeyCode(0)
        public val ANSI_S: KeyCode = KeyCode(1)
        public val ANSI_D: KeyCode = KeyCode(2)
        public val ANSI_F: KeyCode = KeyCode(3)
        public val ANSI_H: KeyCode = KeyCode(4)
        public val ANSI_G: KeyCode = KeyCode(5)
        public val ANSI_Z: KeyCode = KeyCode(6)
        public val ANSI_X: KeyCode = KeyCode(7)
        public val ANSI_C: KeyCode = KeyCode(8)
        public val ANSI_V: KeyCode = KeyCode(9)
        public val ANSI_B: KeyCode = KeyCode(11)
        public val ANSI_Q: KeyCode = KeyCode(12)
        public val ANSI_W: KeyCode = KeyCode(13)
        public val ANSI_E: KeyCode = KeyCode(14)
        public val ANSI_R: KeyCode = KeyCode(15)
        public val ANSI_Y: KeyCode = KeyCode(16)
        public val ANSI_T: KeyCode = KeyCode(17)
        public val ANSI_1: KeyCode = KeyCode(18)
        public val ANSI_2: KeyCode = KeyCode(19)
        public val ANSI_3: KeyCode = KeyCode(20)
        public val ANSI_4: KeyCode = KeyCode(21)
        public val ANSI_6: KeyCode = KeyCode(22)
        public val ANSI_5: KeyCode = KeyCode(23)
        public val ANSI_Equal: KeyCode = KeyCode(24)
        public val ANSI_9: KeyCode = KeyCode(25)
        public val ANSI_7: KeyCode = KeyCode(26)
        public val ANSI_Minus: KeyCode = KeyCode(27)
        public val ANSI_8: KeyCode = KeyCode(28)
        public val ANSI_0: KeyCode = KeyCode(29)
        public val ANSI_RightBracket: KeyCode = KeyCode(30)
        public val ANSI_O: KeyCode = KeyCode(31)
        public val ANSI_U: KeyCode = KeyCode(32)
        public val ANSI_LeftBracket: KeyCode = KeyCode(33)
        public val ANSI_I: KeyCode = KeyCode(34)
        public val ANSI_P: KeyCode = KeyCode(35)
        public val ANSI_L: KeyCode = KeyCode(37)
        public val ANSI_J: KeyCode = KeyCode(38)
        public val ANSI_Quote: KeyCode = KeyCode(39)
        public val ANSI_K: KeyCode = KeyCode(40)
        public val ANSI_Semicolon: KeyCode = KeyCode(41)
        public val ANSI_Backslash: KeyCode = KeyCode(42)
        public val ANSI_Comma: KeyCode = KeyCode(43)
        public val ANSI_Slash: KeyCode = KeyCode(44)
        public val ANSI_N: KeyCode = KeyCode(45)
        public val ANSI_M: KeyCode = KeyCode(46)
        public val ANSI_Period: KeyCode = KeyCode(47)
        public val ANSI_Grave: KeyCode = KeyCode(50)
        public val ANSI_KeypadDecimal: KeyCode = KeyCode(65)
        public val ANSI_KeypadMultiply: KeyCode = KeyCode(67)
        public val ANSI_KeypadPlus: KeyCode = KeyCode(69)
        public val ANSI_KeypadClear: KeyCode = KeyCode(71)
        public val ANSI_KeypadDivide: KeyCode = KeyCode(75)
        public val ANSI_KeypadEnter: KeyCode = KeyCode(76)
        public val ANSI_KeypadMinus: KeyCode = KeyCode(78)
        public val ANSI_KeypadEquals: KeyCode = KeyCode(81)
        public val ANSI_Keypad0: KeyCode = KeyCode(82)
        public val ANSI_Keypad1: KeyCode = KeyCode(83)
        public val ANSI_Keypad2: KeyCode = KeyCode(84)
        public val ANSI_Keypad3: KeyCode = KeyCode(85)
        public val ANSI_Keypad4: KeyCode = KeyCode(86)
        public val ANSI_Keypad5: KeyCode = KeyCode(87)
        public val ANSI_Keypad6: KeyCode = KeyCode(88)
        public val ANSI_Keypad7: KeyCode = KeyCode(89)
        public val ANSI_Keypad8: KeyCode = KeyCode(91)
        public val ANSI_Keypad9: KeyCode = KeyCode(92)

        /* keycodes for keys that are independent of keyboard layout*/
        public val Return: KeyCode = KeyCode(36)
        public val Tab: KeyCode = KeyCode(48)
        public val Space: KeyCode = KeyCode(49)
        public val Delete: KeyCode = KeyCode(51)
        public val Escape: KeyCode = KeyCode(53)
        public val Command: KeyCode = KeyCode(55)
        public val Shift: KeyCode = KeyCode(56)
        public val CapsLock: KeyCode = KeyCode(57)
        public val Option: KeyCode = KeyCode(58)
        public val Control: KeyCode = KeyCode(59)
        public val RightCommand: KeyCode = KeyCode(54)
        public val RightShift: KeyCode = KeyCode(60)
        public val RightOption: KeyCode = KeyCode(61)
        public val RightControl: KeyCode = KeyCode(62)
        public val Function: KeyCode = KeyCode(63)
        public val F17: KeyCode = KeyCode(64)
        public val VolumeUp: KeyCode = KeyCode(72)
        public val VolumeDown: KeyCode = KeyCode(73)
        public val Mute: KeyCode = KeyCode(74)
        public val F18: KeyCode = KeyCode(79)
        public val F19: KeyCode = KeyCode(80)
        public val F20: KeyCode = KeyCode(90)
        public val F5: KeyCode = KeyCode(96)
        public val F6: KeyCode = KeyCode(97)
        public val F7: KeyCode = KeyCode(98)
        public val F3: KeyCode = KeyCode(99)
        public val F8: KeyCode = KeyCode(100)
        public val F9: KeyCode = KeyCode(101)
        public val F11: KeyCode = KeyCode(103)
        public val F13: KeyCode = KeyCode(105)
        public val F16: KeyCode = KeyCode(106)
        public val F14: KeyCode = KeyCode(107)
        public val F10: KeyCode = KeyCode(109)
        public val ContextualMenu: KeyCode = KeyCode(110)
        public val F12: KeyCode = KeyCode(111)
        public val F15: KeyCode = KeyCode(113)
        public val Help: KeyCode = KeyCode(114)
        public val Home: KeyCode = KeyCode(115)
        public val PageUp: KeyCode = KeyCode(116)
        public val ForwardDelete: KeyCode = KeyCode(117)
        public val F4: KeyCode = KeyCode(118)
        public val End: KeyCode = KeyCode(119)
        public val F2: KeyCode = KeyCode(120)
        public val PageDown: KeyCode = KeyCode(121)
        public val F1: KeyCode = KeyCode(122)
        public val LeftArrow: KeyCode = KeyCode(123)
        public val RightArrow: KeyCode = KeyCode(124)
        public val DownArrow: KeyCode = KeyCode(125)
        public val UpArrow: KeyCode = KeyCode(126)

        /* ISO keyboards only*/
        public val ISO_Section: KeyCode = KeyCode(10)

        public val JIS_Yen: KeyCode = KeyCode(93)
        public val JIS_Underscore: KeyCode = KeyCode(94)
        public val JIS_KeypadComma: KeyCode = KeyCode(95)
        public val JIS_Eisu: KeyCode = KeyCode(102)
        public val JIS_Kana: KeyCode = KeyCode(104)

        internal fun fromNative(code: Short): KeyCode {
            return KeyCode(code)
        }
    }

    override fun toString(): String {
        return when (this) {
            ANSI_A -> "ANSI_A"
            ANSI_S -> "ANSI_S"
            ANSI_D -> "ANSI_D"
            ANSI_F -> "ANSI_F"
            ANSI_H -> "ANSI_H"
            ANSI_G -> "ANSI_G"
            ANSI_Z -> "ANSI_Z"
            ANSI_X -> "ANSI_X"
            ANSI_C -> "ANSI_C"
            ANSI_V -> "ANSI_V"
            ANSI_B -> "ANSI_B"
            ANSI_Q -> "ANSI_Q"
            ANSI_W -> "ANSI_W"
            ANSI_E -> "ANSI_E"
            ANSI_R -> "ANSI_R"
            ANSI_Y -> "ANSI_Y"
            ANSI_T -> "ANSI_T"
            ANSI_1 -> "ANSI_1"
            ANSI_2 -> "ANSI_2"
            ANSI_3 -> "ANSI_3"
            ANSI_4 -> "ANSI_4"
            ANSI_6 -> "ANSI_6"
            ANSI_5 -> "ANSI_5"
            ANSI_Equal -> "ANSI_Equal"
            ANSI_9 -> "ANSI_9"
            ANSI_7 -> "ANSI_7"
            ANSI_Minus -> "ANSI_Minus"
            ANSI_8 -> "ANSI_8"
            ANSI_0 -> "ANSI_0"
            ANSI_RightBracket -> "ANSI_RightBracket"
            ANSI_O -> "ANSI_O"
            ANSI_U -> "ANSI_U"
            ANSI_LeftBracket -> "ANSI_LeftBracket"
            ANSI_I -> "ANSI_I"
            ANSI_P -> "ANSI_P"
            ANSI_L -> "ANSI_L"
            ANSI_J -> "ANSI_J"
            ANSI_Quote -> "ANSI_Quote"
            ANSI_K -> "ANSI_K"
            ANSI_Semicolon -> "ANSI_Semicolon"
            ANSI_Backslash -> "ANSI_Backslash"
            ANSI_Comma -> "ANSI_Comma"
            ANSI_Slash -> "ANSI_Slash"
            ANSI_N -> "ANSI_N"
            ANSI_M -> "ANSI_M"
            ANSI_Period -> "ANSI_Period"
            ANSI_Grave -> "ANSI_Grave"
            ANSI_KeypadDecimal -> "ANSI_KeypadDecimal"
            ANSI_KeypadMultiply -> "ANSI_KeypadMultiply"
            ANSI_KeypadPlus -> "ANSI_KeypadPlus"
            ANSI_KeypadClear -> "ANSI_KeypadClear"
            ANSI_KeypadDivide -> "ANSI_KeypadDivide"
            ANSI_KeypadEnter -> "ANSI_KeypadEnter"
            ANSI_KeypadMinus -> "ANSI_KeypadMinus"
            ANSI_KeypadEquals -> "ANSI_KeypadEquals"
            ANSI_Keypad0 -> "ANSI_Keypad0"
            ANSI_Keypad1 -> "ANSI_Keypad1"
            ANSI_Keypad2 -> "ANSI_Keypad2"
            ANSI_Keypad3 -> "ANSI_Keypad3"
            ANSI_Keypad4 -> "ANSI_Keypad4"
            ANSI_Keypad5 -> "ANSI_Keypad5"
            ANSI_Keypad6 -> "ANSI_Keypad6"
            ANSI_Keypad7 -> "ANSI_Keypad7"
            ANSI_Keypad8 -> "ANSI_Keypad8"
            ANSI_Keypad9 -> "ANSI_Keypad9"
            Return -> "Return"
            Tab -> "Tab"
            Space -> "Space"
            Delete -> "Delete"
            Escape -> "Escape"
            Command -> "Command"
            Shift -> "Shift"
            CapsLock -> "CapsLock"
            Option -> "Option"
            Control -> "Control"
            RightCommand -> "RightCommand"
            RightShift -> "RightShift"
            RightOption -> "RightOption"
            RightControl -> "RightControl"
            Function -> "Function"
            F17 -> "F17"
            VolumeUp -> "VolumeUp"
            VolumeDown -> "VolumeDown"
            Mute -> "Mute"
            F18 -> "F18"
            F19 -> "F19"
            F20 -> "F20"
            F5 -> "F5"
            F6 -> "F6"
            F7 -> "F7"
            F3 -> "F3"
            F8 -> "F8"
            F9 -> "F9"
            F11 -> "F11"
            F13 -> "F13"
            F16 -> "F16"
            F14 -> "F14"
            F10 -> "F10"
            ContextualMenu -> "ContextualMenu"
            F12 -> "F12"
            F15 -> "F15"
            Help -> "Help"
            Home -> "Home"
            PageUp -> "PageUp"
            ForwardDelete -> "ForwardDelete"
            F4 -> "F4"
            End -> "End"
            F2 -> "F2"
            PageDown -> "PageDown"
            F1 -> "F1"
            LeftArrow -> "LeftArrow"
            RightArrow -> "RightArrow"
            DownArrow -> "DownArrow"
            UpArrow -> "UpArrow"
            ISO_Section -> "ISO_Section"
            JIS_Yen -> "JIS_Yen"
            JIS_Underscore -> "JIS_Underscore"
            JIS_KeypadComma -> "JIS_KeypadComma"
            JIS_Eisu -> "JIS_Eisu"
            JIS_Kana -> "JIS_Kana"
            else -> {
                "UNKNOWN"
            }
        }
    }
}

// `NSEventModifierFlags` constants
private object KeyModifiers {
    const val CAPS_LOCK: Long = 1 shl 16
    const val SHIFT: Long = 1 shl 17
    const val CONTROL: Long = 1 shl 18
    const val OPTION: Long = 1 shl 19
    const val COMMAND: Long = 1 shl 20
    const val NUMERIC_PAD: Long = 1 shl 21
    const val HELP: Long = 1 shl 22
    const val FUNCTION: Long = 1 shl 23
}

@Suppress("MemberVisibilityCanBePrivate")
@JvmInline
public value class KeyModifiersSet internal constructor(internal val value: Long) {
    public companion object {
        public fun create(
            capsLock: Boolean = false,
            shift: Boolean = false,
            control: Boolean = false,
            option: Boolean = false,
            command: Boolean = false,
            numericPad: Boolean = false,
            help: Boolean = false,
            function: Boolean = false,
        ): KeyModifiersSet {
            var result = 0L
            if (capsLock) result = result or KeyModifiers.CAPS_LOCK
            if (shift) result = result or KeyModifiers.SHIFT
            if (control) result = result or KeyModifiers.CONTROL
            if (option) result = result or KeyModifiers.OPTION
            if (command) result = result or KeyModifiers.COMMAND
            if (numericPad) result = result or KeyModifiers.NUMERIC_PAD
            if (help) result = result or KeyModifiers.HELP
            if (function) result = result or KeyModifiers.FUNCTION
            return KeyModifiersSet(result)
        }
    }

    public val capsLock: Boolean get() = (value and KeyModifiers.CAPS_LOCK) != 0L
    public val shift: Boolean get() = (value and KeyModifiers.SHIFT) != 0L
    public val control: Boolean get() = (value and KeyModifiers.CONTROL) != 0L
    public val option: Boolean get() = (value and KeyModifiers.OPTION) != 0L
    public val command: Boolean get() = (value and KeyModifiers.COMMAND) != 0L
    public val numericPad: Boolean get() = (value and KeyModifiers.NUMERIC_PAD) != 0L
    public val help: Boolean get() = (value and KeyModifiers.HELP) != 0L
    public val function: Boolean get() = (value and KeyModifiers.FUNCTION) != 0L

    override fun toString(): String {
        val modifiers = buildList {
            if (capsLock) add("CapsLock")
            if (shift) add("Shift")
            if (control) add("Control")
            if (option) add("Option")
            if (command) add("Command")
            if (numericPad) add("NumericPad")
            if (help) add("Help")
            if (function) add("Function")
        }
        return "KeyModifiersSet($modifiers)"
    }
}

private fun String.toCodepointsString(): String {
    return codePoints().toArray().joinToString(prefix = "[", postfix = "]") { "U+${it.toString(16).uppercase().padStart(4, '0')}" }
}

@JvmInline
public value class Characters internal constructor(public val text: String) {
    public val specialKey: SpecialKey?
        get() {
            val codepoints = text.codePoints().toArray()
            if (codepoints.size == 1) {
                return SpecialKey.fromCodepoint(codepoints[0])
            }
            return null
        }

    public val specialCharacter: SpecialCharacter?
        get() {
            val codepoints = text.codePoints().toArray()
            if (codepoints.size == 1) {
                return SpecialCharacter.fromCodepoint(codepoints[0])
            }
            return null
        }

    override fun toString(): String {
        val specialKey = this.specialKey
        val specialCharacter = this.specialCharacter
        return specialKey?.toString()
            ?: specialCharacter?.toString()
            ?: "$text|${text.toCodepointsString()}"
    }
}

/**
 * Those Unicode characters are used by [Event.KeyDown.characters] [Event.KeyDown.charactersIgnoringModifiers],
 * [Event.KeyDown.key] [Event.KeyDown.keyWithModifiers] or [Event.charactersByApplyingModifiersForCurrentEvent]
 * but be aware that the same button might produce different values for [Event.KeyDown.charactersIgnoringModifiers]
 * and [Event.KeyDown.key]
 */
@JvmInline
public value class SpecialCharacter private constructor(public val codepoint: Int) {
    public companion object {
        public val EnterCharacter: SpecialCharacter = SpecialCharacter(0x0003)
        public val BackspaceCharacter: SpecialCharacter = SpecialCharacter(0x0008)
        public val TabCharacter: SpecialCharacter = SpecialCharacter(0x0009)
        public val NewlineCharacter: SpecialCharacter = SpecialCharacter(0x000a)
        public val FormFeedCharacter: SpecialCharacter = SpecialCharacter(0x000c)
        public val CarriageReturnCharacter: SpecialCharacter = SpecialCharacter(0x000d)
        public val BackTabCharacter: SpecialCharacter = SpecialCharacter(0x0019) // Tab with Shift
        public val DeleteCharacter: SpecialCharacter = SpecialCharacter(0x007f)
        public val LineSeparatorCharacter: SpecialCharacter = SpecialCharacter(0x2028)
        public val ParagraphSeparatorCharacter: SpecialCharacter = SpecialCharacter(0x2029)
        public val SpaceCharacter: SpecialCharacter = SpecialCharacter(0x0020)
        public val EscapeCharacter: SpecialCharacter = SpecialCharacter(0x001b)

        internal fun fromCodepoint(codepoint: Int): SpecialCharacter? {
            return when (codepoint) {
                EnterCharacter.codepoint -> EnterCharacter
                BackspaceCharacter.codepoint -> BackspaceCharacter
                TabCharacter.codepoint -> TabCharacter
                NewlineCharacter.codepoint -> NewlineCharacter
                FormFeedCharacter.codepoint -> FormFeedCharacter
                CarriageReturnCharacter.codepoint -> CarriageReturnCharacter
                BackTabCharacter.codepoint -> BackTabCharacter
                DeleteCharacter.codepoint -> DeleteCharacter
                LineSeparatorCharacter.codepoint -> LineSeparatorCharacter
                ParagraphSeparatorCharacter.codepoint -> ParagraphSeparatorCharacter
                SpaceCharacter.codepoint -> SpaceCharacter
                EscapeCharacter.codepoint -> EscapeCharacter
                else -> null
            }
        }
    }

    override fun toString(): String {
        return when (this) {
            EnterCharacter -> "EnterCharacter"
            BackspaceCharacter -> "BackspaceCharacter"
            TabCharacter -> "TabCharacter"
            NewlineCharacter -> "NewlineCharacter"
            FormFeedCharacter -> "FormFeedCharacter"
            CarriageReturnCharacter -> "CarriageReturnCharacter"
            BackTabCharacter -> "BackTabCharacter"
            DeleteCharacter -> "DeleteCharacter"
            LineSeparatorCharacter -> "LineSeparatorCharacter"
            ParagraphSeparatorCharacter -> "ParagraphSeparatorCharacter"
            SpaceCharacter -> "SpaceCharacter"
            EscapeCharacter -> "EscapeCharacter"
            else -> throw IllegalStateException()
        }
    }
}

/**
 * It's Unicode codepoints from a private plane reserved by apple to represent some non-typing keys on the keyboard.
 * It's used by [Event.KeyDown.characters] and [Event.KeyDown.charactersIgnoringModifiers],
 * but not by [Event.KeyDown.key] and [Event.KeyDown.keyWithModifiers] or [Event.charactersByApplyingModifiersForCurrentEvent]
 */
@JvmInline
public value class SpecialKey private constructor(public val codepoint: Int) {
    public companion object {
        // Unicode private use area
        public val UpArrowFunctionKey: SpecialKey = SpecialKey(0xF700)
        public val DownArrowFunctionKey: SpecialKey = SpecialKey(0xF701)
        public val LeftArrowFunctionKey: SpecialKey = SpecialKey(0xF702)
        public val RightArrowFunctionKey: SpecialKey = SpecialKey(0xF703)
        public val F1FunctionKey: SpecialKey = SpecialKey(0xF704)
        public val F2FunctionKey: SpecialKey = SpecialKey(0xF705)
        public val F3FunctionKey: SpecialKey = SpecialKey(0xF706)
        public val F4FunctionKey: SpecialKey = SpecialKey(0xF707)
        public val F5FunctionKey: SpecialKey = SpecialKey(0xF708)
        public val F6FunctionKey: SpecialKey = SpecialKey(0xF709)
        public val F7FunctionKey: SpecialKey = SpecialKey(0xF70A)
        public val F8FunctionKey: SpecialKey = SpecialKey(0xF70B)
        public val F9FunctionKey: SpecialKey = SpecialKey(0xF70C)
        public val F10FunctionKey: SpecialKey = SpecialKey(0xF70D)
        public val F11FunctionKey: SpecialKey = SpecialKey(0xF70E)
        public val F12FunctionKey: SpecialKey = SpecialKey(0xF70F)
        public val F13FunctionKey: SpecialKey = SpecialKey(0xF710)
        public val F14FunctionKey: SpecialKey = SpecialKey(0xF711)
        public val F15FunctionKey: SpecialKey = SpecialKey(0xF712)
        public val F16FunctionKey: SpecialKey = SpecialKey(0xF713)
        public val F17FunctionKey: SpecialKey = SpecialKey(0xF714)
        public val F18FunctionKey: SpecialKey = SpecialKey(0xF715)
        public val F19FunctionKey: SpecialKey = SpecialKey(0xF716)
        public val F20FunctionKey: SpecialKey = SpecialKey(0xF717)
        public val F21FunctionKey: SpecialKey = SpecialKey(0xF718)
        public val F22FunctionKey: SpecialKey = SpecialKey(0xF719)
        public val F23FunctionKey: SpecialKey = SpecialKey(0xF71A)
        public val F24FunctionKey: SpecialKey = SpecialKey(0xF71B)
        public val F25FunctionKey: SpecialKey = SpecialKey(0xF71C)
        public val F26FunctionKey: SpecialKey = SpecialKey(0xF71D)
        public val F27FunctionKey: SpecialKey = SpecialKey(0xF71E)
        public val F28FunctionKey: SpecialKey = SpecialKey(0xF71F)
        public val F29FunctionKey: SpecialKey = SpecialKey(0xF720)
        public val F30FunctionKey: SpecialKey = SpecialKey(0xF721)
        public val F31FunctionKey: SpecialKey = SpecialKey(0xF722)
        public val F32FunctionKey: SpecialKey = SpecialKey(0xF723)
        public val F33FunctionKey: SpecialKey = SpecialKey(0xF724)
        public val F34FunctionKey: SpecialKey = SpecialKey(0xF725)
        public val F35FunctionKey: SpecialKey = SpecialKey(0xF726)
        public val InsertFunctionKey: SpecialKey = SpecialKey(0xF727)
        public val DeleteFunctionKey: SpecialKey = SpecialKey(0xF728)
        public val HomeFunctionKey: SpecialKey = SpecialKey(0xF729)
        public val BeginFunctionKey: SpecialKey = SpecialKey(0xF72A)
        public val EndFunctionKey: SpecialKey = SpecialKey(0xF72B)
        public val PageUpFunctionKey: SpecialKey = SpecialKey(0xF72C)
        public val PageDownFunctionKey: SpecialKey = SpecialKey(0xF72D)
        public val PrintScreenFunctionKey: SpecialKey = SpecialKey(0xF72E)
        public val ScrollLockFunctionKey: SpecialKey = SpecialKey(0xF72F)
        public val PauseFunctionKey: SpecialKey = SpecialKey(0xF730)
        public val SysReqFunctionKey: SpecialKey = SpecialKey(0xF731)
        public val BreakFunctionKey: SpecialKey = SpecialKey(0xF732)
        public val ResetFunctionKey: SpecialKey = SpecialKey(0xF733)
        public val StopFunctionKey: SpecialKey = SpecialKey(0xF734)
        public val MenuFunctionKey: SpecialKey = SpecialKey(0xF735)
        public val UserFunctionKey: SpecialKey = SpecialKey(0xF736)
        public val SystemFunctionKey: SpecialKey = SpecialKey(0xF737)
        public val PrintFunctionKey: SpecialKey = SpecialKey(0xF738)
        public val ClearLineFunctionKey: SpecialKey = SpecialKey(0xF739)
        public val ClearDisplayFunctionKey: SpecialKey = SpecialKey(0xF73A)
        public val InsertLineFunctionKey: SpecialKey = SpecialKey(0xF73B)
        public val DeleteLineFunctionKey: SpecialKey = SpecialKey(0xF73C)
        public val InsertCharFunctionKey: SpecialKey = SpecialKey(0xF73D)
        public val DeleteCharFunctionKey: SpecialKey = SpecialKey(0xF73E)
        public val PrevFunctionKey: SpecialKey = SpecialKey(0xF73F)
        public val NextFunctionKey: SpecialKey = SpecialKey(0xF740)
        public val SelectFunctionKey: SpecialKey = SpecialKey(0xF741)
        public val ExecuteFunctionKey: SpecialKey = SpecialKey(0xF742)
        public val UndoFunctionKey: SpecialKey = SpecialKey(0xF743)
        public val RedoFunctionKey: SpecialKey = SpecialKey(0xF744)
        public val FindFunctionKey: SpecialKey = SpecialKey(0xF745)
        public val HelpFunctionKey: SpecialKey = SpecialKey(0xF746)
        public val ModeSwitchFunctionKey: SpecialKey = SpecialKey(0xF747)

        public fun fromCodepoint(codepoint: Int): SpecialKey? {
            return when (codepoint) {
                UpArrowFunctionKey.codepoint -> UpArrowFunctionKey
                DownArrowFunctionKey.codepoint -> DownArrowFunctionKey
                LeftArrowFunctionKey.codepoint -> LeftArrowFunctionKey
                RightArrowFunctionKey.codepoint -> RightArrowFunctionKey
                F1FunctionKey.codepoint -> F1FunctionKey
                F2FunctionKey.codepoint -> F2FunctionKey
                F3FunctionKey.codepoint -> F3FunctionKey
                F4FunctionKey.codepoint -> F4FunctionKey
                F5FunctionKey.codepoint -> F5FunctionKey
                F6FunctionKey.codepoint -> F6FunctionKey
                F7FunctionKey.codepoint -> F7FunctionKey
                F8FunctionKey.codepoint -> F8FunctionKey
                F9FunctionKey.codepoint -> F9FunctionKey
                F10FunctionKey.codepoint -> F10FunctionKey
                F11FunctionKey.codepoint -> F11FunctionKey
                F12FunctionKey.codepoint -> F12FunctionKey
                F13FunctionKey.codepoint -> F13FunctionKey
                F14FunctionKey.codepoint -> F14FunctionKey
                F15FunctionKey.codepoint -> F15FunctionKey
                F16FunctionKey.codepoint -> F16FunctionKey
                F17FunctionKey.codepoint -> F17FunctionKey
                F18FunctionKey.codepoint -> F18FunctionKey
                F19FunctionKey.codepoint -> F19FunctionKey
                F20FunctionKey.codepoint -> F20FunctionKey
                F21FunctionKey.codepoint -> F21FunctionKey
                F22FunctionKey.codepoint -> F22FunctionKey
                F23FunctionKey.codepoint -> F23FunctionKey
                F24FunctionKey.codepoint -> F24FunctionKey
                F25FunctionKey.codepoint -> F25FunctionKey
                F26FunctionKey.codepoint -> F26FunctionKey
                F27FunctionKey.codepoint -> F27FunctionKey
                F28FunctionKey.codepoint -> F28FunctionKey
                F29FunctionKey.codepoint -> F29FunctionKey
                F30FunctionKey.codepoint -> F30FunctionKey
                F31FunctionKey.codepoint -> F31FunctionKey
                F32FunctionKey.codepoint -> F32FunctionKey
                F33FunctionKey.codepoint -> F33FunctionKey
                F34FunctionKey.codepoint -> F34FunctionKey
                F35FunctionKey.codepoint -> F35FunctionKey
                InsertFunctionKey.codepoint -> InsertFunctionKey
                DeleteFunctionKey.codepoint -> DeleteFunctionKey
                HomeFunctionKey.codepoint -> HomeFunctionKey
                BeginFunctionKey.codepoint -> BeginFunctionKey
                EndFunctionKey.codepoint -> EndFunctionKey
                PageUpFunctionKey.codepoint -> PageUpFunctionKey
                PageDownFunctionKey.codepoint -> PageDownFunctionKey
                PrintScreenFunctionKey.codepoint -> PrintScreenFunctionKey
                ScrollLockFunctionKey.codepoint -> ScrollLockFunctionKey
                PauseFunctionKey.codepoint -> PauseFunctionKey
                SysReqFunctionKey.codepoint -> SysReqFunctionKey
                BreakFunctionKey.codepoint -> BreakFunctionKey
                ResetFunctionKey.codepoint -> ResetFunctionKey
                StopFunctionKey.codepoint -> StopFunctionKey
                MenuFunctionKey.codepoint -> MenuFunctionKey
                UserFunctionKey.codepoint -> UserFunctionKey
                SystemFunctionKey.codepoint -> SystemFunctionKey
                PrintFunctionKey.codepoint -> PrintFunctionKey
                ClearLineFunctionKey.codepoint -> ClearLineFunctionKey
                ClearDisplayFunctionKey.codepoint -> ClearDisplayFunctionKey
                InsertLineFunctionKey.codepoint -> InsertLineFunctionKey
                DeleteLineFunctionKey.codepoint -> DeleteLineFunctionKey
                InsertCharFunctionKey.codepoint -> InsertCharFunctionKey
                DeleteCharFunctionKey.codepoint -> DeleteCharFunctionKey
                PrevFunctionKey.codepoint -> PrevFunctionKey
                NextFunctionKey.codepoint -> NextFunctionKey
                SelectFunctionKey.codepoint -> SelectFunctionKey
                ExecuteFunctionKey.codepoint -> ExecuteFunctionKey
                UndoFunctionKey.codepoint -> UndoFunctionKey
                RedoFunctionKey.codepoint -> RedoFunctionKey
                FindFunctionKey.codepoint -> FindFunctionKey
                HelpFunctionKey.codepoint -> HelpFunctionKey
                ModeSwitchFunctionKey.codepoint -> ModeSwitchFunctionKey
                else -> null
            }
        }
    }

    override fun toString(): String {
        return when (this) {
            UpArrowFunctionKey -> "UpArrowFunctionKey"
            DownArrowFunctionKey -> "DownArrowFunctionKey"
            LeftArrowFunctionKey -> "LeftArrowFunctionKey"
            RightArrowFunctionKey -> "RightArrowFunctionKey"
            F1FunctionKey -> "F1FunctionKey"
            F2FunctionKey -> "F2FunctionKey"
            F3FunctionKey -> "F3FunctionKey"
            F4FunctionKey -> "F4FunctionKey"
            F5FunctionKey -> "F5FunctionKey"
            F6FunctionKey -> "F6FunctionKey"
            F7FunctionKey -> "F7FunctionKey"
            F8FunctionKey -> "F8FunctionKey"
            F9FunctionKey -> "F9FunctionKey"
            F10FunctionKey -> "F10FunctionKey"
            F11FunctionKey -> "F11FunctionKey"
            F12FunctionKey -> "F12FunctionKey"
            F13FunctionKey -> "F13FunctionKey"
            F14FunctionKey -> "F14FunctionKey"
            F15FunctionKey -> "F15FunctionKey"
            F16FunctionKey -> "F16FunctionKey"
            F17FunctionKey -> "F17FunctionKey"
            F18FunctionKey -> "F18FunctionKey"
            F19FunctionKey -> "F19FunctionKey"
            F20FunctionKey -> "F20FunctionKey"
            F21FunctionKey -> "F21FunctionKey"
            F22FunctionKey -> "F22FunctionKey"
            F23FunctionKey -> "F23FunctionKey"
            F24FunctionKey -> "F24FunctionKey"
            F25FunctionKey -> "F25FunctionKey"
            F26FunctionKey -> "F26FunctionKey"
            F27FunctionKey -> "F27FunctionKey"
            F28FunctionKey -> "F28FunctionKey"
            F29FunctionKey -> "F29FunctionKey"
            F30FunctionKey -> "F30FunctionKey"
            F31FunctionKey -> "F31FunctionKey"
            F32FunctionKey -> "F32FunctionKey"
            F33FunctionKey -> "F33FunctionKey"
            F34FunctionKey -> "F34FunctionKey"
            F35FunctionKey -> "F35FunctionKey"
            InsertFunctionKey -> "InsertFunctionKey"
            DeleteFunctionKey -> "DeleteFunctionKey"
            HomeFunctionKey -> "HomeFunctionKey"
            BeginFunctionKey -> "BeginFunctionKey"
            EndFunctionKey -> "EndFunctionKey"
            PageUpFunctionKey -> "PageUpFunctionKey"
            PageDownFunctionKey -> "PageDownFunctionKey"
            PrintScreenFunctionKey -> "PrintScreenFunctionKey"
            ScrollLockFunctionKey -> "ScrollLockFunctionKey"
            PauseFunctionKey -> "PauseFunctionKey"
            SysReqFunctionKey -> "SysReqFunctionKey"
            BreakFunctionKey -> "BreakFunctionKey"
            ResetFunctionKey -> "ResetFunctionKey"
            StopFunctionKey -> "StopFunctionKey"
            MenuFunctionKey -> "MenuFunctionKey"
            UserFunctionKey -> "UserFunctionKey"
            SystemFunctionKey -> "SystemFunctionKey"
            PrintFunctionKey -> "PrintFunctionKey"
            ClearLineFunctionKey -> "ClearLineFunctionKey"
            ClearDisplayFunctionKey -> "ClearDisplayFunctionKey"
            InsertLineFunctionKey -> "InsertLineFunctionKey"
            DeleteLineFunctionKey -> "DeleteLineFunctionKey"
            InsertCharFunctionKey -> "InsertCharFunctionKey"
            DeleteCharFunctionKey -> "DeleteCharFunctionKey"
            PrevFunctionKey -> "PrevFunctionKey"
            NextFunctionKey -> "NextFunctionKey"
            SelectFunctionKey -> "SelectFunctionKey"
            ExecuteFunctionKey -> "ExecuteFunctionKey"
            UndoFunctionKey -> "UndoFunctionKey"
            RedoFunctionKey -> "RedoFunctionKey"
            FindFunctionKey -> "FindFunctionKey"
            HelpFunctionKey -> "HelpFunctionKey"
            ModeSwitchFunctionKey -> "ModeSwitchFunctionKey"
            else -> throw IllegalStateException()
        }
    }
}
