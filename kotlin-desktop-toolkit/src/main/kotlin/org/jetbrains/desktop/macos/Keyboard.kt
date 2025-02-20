package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

/*  MacOSX15.2
* 🚨🚨🚨 This code should be aligned with `typedef enum KeyCode` in .h file 🚨🚨🚨
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
public value class KeyCode internal constructor(private val value: String) {
    @Suppress("MemberVisibilityCanBePrivate")
    public companion object {
        public val ANSI_A: KeyCode = KeyCode("ANSI_A")
        public val ANSI_S: KeyCode = KeyCode("ANSI_S")
        public val ANSI_D: KeyCode = KeyCode("ANSI_D")
        public val ANSI_F: KeyCode = KeyCode("ANSI_F")
        public val ANSI_H: KeyCode = KeyCode("ANSI_H")
        public val ANSI_G: KeyCode = KeyCode("ANSI_G")
        public val ANSI_Z: KeyCode = KeyCode("ANSI_Z")
        public val ANSI_X: KeyCode = KeyCode("ANSI_X")
        public val ANSI_C: KeyCode = KeyCode("ANSI_C")
        public val ANSI_V: KeyCode = KeyCode("ANSI_V")
        public val ANSI_B: KeyCode = KeyCode("ANSI_B")
        public val ANSI_Q: KeyCode = KeyCode("ANSI_Q")
        public val ANSI_W: KeyCode = KeyCode("ANSI_W")
        public val ANSI_E: KeyCode = KeyCode("ANSI_E")
        public val ANSI_R: KeyCode = KeyCode("ANSI_R")
        public val ANSI_Y: KeyCode = KeyCode("ANSI_Y")
        public val ANSI_T: KeyCode = KeyCode("ANSI_T")
        public val ANSI_1: KeyCode = KeyCode("ANSI_1")
        public val ANSI_2: KeyCode = KeyCode("ANSI_2")
        public val ANSI_3: KeyCode = KeyCode("ANSI_3")
        public val ANSI_4: KeyCode = KeyCode("ANSI_4")
        public val ANSI_6: KeyCode = KeyCode("ANSI_6")
        public val ANSI_5: KeyCode = KeyCode("ANSI_5")
        public val ANSI_Equal: KeyCode = KeyCode("ANSI_Equal")
        public val ANSI_9: KeyCode = KeyCode("ANSI_9")
        public val ANSI_7: KeyCode = KeyCode("ANSI_7")
        public val ANSI_Minus: KeyCode = KeyCode("ANSI_Minus")
        public val ANSI_8: KeyCode = KeyCode("ANSI_8")
        public val ANSI_0: KeyCode = KeyCode("ANSI_0")
        public val ANSI_RightBracket: KeyCode = KeyCode("ANSI_RightBracket")
        public val ANSI_O: KeyCode = KeyCode("ANSI_O")
        public val ANSI_U: KeyCode = KeyCode("ANSI_U")
        public val ANSI_LeftBracket: KeyCode = KeyCode("ANSI_LeftBracket")
        public val ANSI_I: KeyCode = KeyCode("ANSI_I")
        public val ANSI_P: KeyCode = KeyCode("ANSI_P")
        public val ANSI_L: KeyCode = KeyCode("ANSI_L")
        public val ANSI_J: KeyCode = KeyCode("ANSI_J")
        public val ANSI_Quote: KeyCode = KeyCode("ANSI_Quote")
        public val ANSI_K: KeyCode = KeyCode("ANSI_K")
        public val ANSI_Semicolon: KeyCode = KeyCode("ANSI_Semicolon")
        public val ANSI_Backslash: KeyCode = KeyCode("ANSI_Backslash")
        public val ANSI_Comma: KeyCode = KeyCode("ANSI_Comma")
        public val ANSI_Slash: KeyCode = KeyCode("ANSI_Slash")
        public val ANSI_N: KeyCode = KeyCode("ANSI_N")
        public val ANSI_M: KeyCode = KeyCode("ANSI_M")
        public val ANSI_Period: KeyCode = KeyCode("ANSI_Period")
        public val ANSI_Grave: KeyCode = KeyCode("ANSI_Grave")
        public val ANSI_KeypadDecimal: KeyCode = KeyCode("ANSI_KeypadDecimal")
        public val ANSI_KeypadMultiply: KeyCode = KeyCode("ANSI_KeypadMultiply")
        public val ANSI_KeypadPlus: KeyCode = KeyCode("ANSI_KeypadPlus")
        public val ANSI_KeypadClear: KeyCode = KeyCode("ANSI_KeypadClear")
        public val ANSI_KeypadDivide: KeyCode = KeyCode("ANSI_KeypadDivide")
        public val ANSI_KeypadEnter: KeyCode = KeyCode("ANSI_KeypadEnter")
        public val ANSI_KeypadMinus: KeyCode = KeyCode("ANSI_KeypadMinus")
        public val ANSI_KeypadEquals: KeyCode = KeyCode("ANSI_KeypadEquals")
        public val ANSI_Keypad0: KeyCode = KeyCode("ANSI_Keypad0")
        public val ANSI_Keypad1: KeyCode = KeyCode("ANSI_Keypad1")
        public val ANSI_Keypad2: KeyCode = KeyCode("ANSI_Keypad2")
        public val ANSI_Keypad3: KeyCode = KeyCode("ANSI_Keypad3")
        public val ANSI_Keypad4: KeyCode = KeyCode("ANSI_Keypad4")
        public val ANSI_Keypad5: KeyCode = KeyCode("ANSI_Keypad5")
        public val ANSI_Keypad6: KeyCode = KeyCode("ANSI_Keypad6")
        public val ANSI_Keypad7: KeyCode = KeyCode("ANSI_Keypad7")
        public val ANSI_Keypad8: KeyCode = KeyCode("ANSI_Keypad8")
        public val ANSI_Keypad9: KeyCode = KeyCode("ANSI_Keypad9")

        /* keycodes for keys that are independent of keyboard layout*/
        public val Return: KeyCode = KeyCode("Return")
        public val Tab: KeyCode = KeyCode("Tab")
        public val Space: KeyCode = KeyCode("Space")
        public val Delete: KeyCode = KeyCode("Delete")
        public val Escape: KeyCode = KeyCode("Escape")
        public val Command: KeyCode = KeyCode("Command")
        public val Shift: KeyCode = KeyCode("Shift")
        public val CapsLock: KeyCode = KeyCode("CapsLock")
        public val Option: KeyCode = KeyCode("Option")
        public val Control: KeyCode = KeyCode("Control")
        public val RightCommand: KeyCode = KeyCode("RightCommand")
        public val RightShift: KeyCode = KeyCode("RightShift")
        public val RightOption: KeyCode = KeyCode("RightOption")
        public val RightControl: KeyCode = KeyCode("RightControl")
        public val Function: KeyCode = KeyCode("Function")
        public val F17: KeyCode = KeyCode("F17")
        public val VolumeUp: KeyCode = KeyCode("VolumeUp")
        public val VolumeDown: KeyCode = KeyCode("VolumeDown")
        public val Mute: KeyCode = KeyCode("Mute")
        public val F18: KeyCode = KeyCode("F18")
        public val F19: KeyCode = KeyCode("F19")
        public val F20: KeyCode = KeyCode("F20")
        public val F5: KeyCode = KeyCode("F5")
        public val F6: KeyCode = KeyCode("F6")
        public val F7: KeyCode = KeyCode("F7")
        public val F3: KeyCode = KeyCode("F3")
        public val F8: KeyCode = KeyCode("F8")
        public val F9: KeyCode = KeyCode("F9")
        public val F11: KeyCode = KeyCode("F11")
        public val F13: KeyCode = KeyCode("F13")
        public val F16: KeyCode = KeyCode("F16")
        public val F14: KeyCode = KeyCode("F14")
        public val F10: KeyCode = KeyCode("F10")
        public val ContextualMenu: KeyCode = KeyCode("ContextualMenu")
        public val F12: KeyCode = KeyCode("F12")
        public val F15: KeyCode = KeyCode("F15")
        public val Help: KeyCode = KeyCode("Help")
        public val Home: KeyCode = KeyCode("Home")
        public val PageUp: KeyCode = KeyCode("PageUp")
        public val ForwardDelete: KeyCode = KeyCode("ForwardDelete")
        public val F4: KeyCode = KeyCode("F4")
        public val End: KeyCode = KeyCode("End")
        public val F2: KeyCode = KeyCode("F2")
        public val PageDown: KeyCode = KeyCode("PageDown")
        public val F1: KeyCode = KeyCode("F1")
        public val LeftArrow: KeyCode = KeyCode("LeftArrow")
        public val RightArrow: KeyCode = KeyCode("RightArrow")
        public val DownArrow: KeyCode = KeyCode("DownArrow")
        public val UpArrow: KeyCode = KeyCode("UpArrow")

        /* ISO keyboards only*/
        public val ISO_Section: KeyCode = KeyCode("ISO_Section")

        public val JIS_Yen: KeyCode = KeyCode("JIS_Yen")
        public val JIS_Underscore: KeyCode = KeyCode("JIS_Underscore")
        public val JIS_KeypadComma: KeyCode = KeyCode("JIS_KeypadComma")
        public val JIS_Eisu: KeyCode = KeyCode("JIS_Eisu")
        public val JIS_Kana: KeyCode = KeyCode("JIS_Kana")

        // This function should be in sync with typedef enum KeyCode
        internal fun fromNative(code: Int): KeyCode {
            return when (code) {
                0 -> ANSI_A
                1 -> ANSI_S
                2 -> ANSI_D
                3 -> ANSI_F
                4 -> ANSI_H
                5 -> ANSI_G
                6 -> ANSI_Z
                7 -> ANSI_X
                8 -> ANSI_C
                9 -> ANSI_V
                11 -> ANSI_B
                12 -> ANSI_Q
                13 -> ANSI_W
                14 -> ANSI_E
                15 -> ANSI_R
                16 -> ANSI_Y
                17 -> ANSI_T
                18 -> ANSI_1
                19 -> ANSI_2
                20 -> ANSI_3
                21 -> ANSI_4
                22 -> ANSI_6
                23 -> ANSI_5
                24 -> ANSI_Equal
                25 -> ANSI_9
                26 -> ANSI_7
                27 -> ANSI_Minus
                28 -> ANSI_8
                29 -> ANSI_0
                30 -> ANSI_RightBracket
                31 -> ANSI_O
                32 -> ANSI_U
                33 -> ANSI_LeftBracket
                34 -> ANSI_I
                35 -> ANSI_P
                37 -> ANSI_L
                38 -> ANSI_J
                39 -> ANSI_Quote
                40 -> ANSI_K
                41 -> ANSI_Semicolon
                42 -> ANSI_Backslash
                43 -> ANSI_Comma
                44 -> ANSI_Slash
                45 -> ANSI_N
                46 -> ANSI_M
                47 -> ANSI_Period
                50 -> ANSI_Grave
                65 -> ANSI_KeypadDecimal
                67 -> ANSI_KeypadMultiply
                69 -> ANSI_KeypadPlus
                71 -> ANSI_KeypadClear
                75 -> ANSI_KeypadDivide
                76 -> ANSI_KeypadEnter
                78 -> ANSI_KeypadMinus
                81 -> ANSI_KeypadEquals
                82 -> ANSI_Keypad0
                83 -> ANSI_Keypad1
                84 -> ANSI_Keypad2
                85 -> ANSI_Keypad3
                86 -> ANSI_Keypad4
                87 -> ANSI_Keypad5
                88 -> ANSI_Keypad6
                89 -> ANSI_Keypad7
                91 -> ANSI_Keypad8
                92 -> ANSI_Keypad9
                36 -> Return
                48 -> Tab
                49 -> Space
                51 -> Delete
                53 -> Escape
                55 -> Command
                56 -> Shift
                57 -> CapsLock
                58 -> Option
                59 -> Control
                54 -> RightCommand
                60 -> RightShift
                61 -> RightOption
                62 -> RightControl
                63 -> Function
                64 -> F17
                72 -> VolumeUp
                73 -> VolumeDown
                74 -> Mute
                79 -> F18
                80 -> F19
                90 -> F20
                96 -> F5
                97 -> F6
                98 -> F7
                99 -> F3
                100 -> F8
                101 -> F9
                103 -> F11
                105 -> F13
                106 -> F16
                107 -> F14
                109 -> F10
                110 -> ContextualMenu
                111 -> F12
                113 -> F15
                114 -> Help
                115 -> Home
                116 -> PageUp
                117 -> ForwardDelete
                118 -> F4
                119 -> End
                120 -> F2
                121 -> PageDown
                122 -> F1
                123 -> LeftArrow
                124 -> RightArrow
                125 -> DownArrow
                126 -> UpArrow
                10 -> ISO_Section
                93 -> JIS_Yen
                94 -> JIS_Underscore
                95 -> JIS_KeypadComma
                102 -> JIS_Eisu
                104 -> JIS_Kana
                else -> {
                    val keyCode = KeyCode("Unknown($code)")
                    Logger.warn { "Got unknown keycode: $keyCode" }
                    keyCode
                }
            }
        }
    }
}

@Suppress("MemberVisibilityCanBePrivate")
@JvmInline
public value class KeyModifiersSet internal constructor(internal val value: Int) {
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
            var result = 0
            if (capsLock) result = result or desktop_macos_h.NativeCapsLockModifier()
            if (shift) result = result or desktop_macos_h.NativeShiftModifier()
            if (control) result = result or desktop_macos_h.NativeControlModifier()
            if (option) result = result or desktop_macos_h.NativeOptionModifier()
            if (command) result = result or desktop_macos_h.NativeCommandModifier()
            if (numericPad) result = result or desktop_macos_h.NativeNumericPadModifier()
            if (help) result = result or desktop_macos_h.NativeHelpModifier()
            if (function) result = result or desktop_macos_h.NativeFunctionModifier()
            return KeyModifiersSet(result)
        }
    }

    public val capsLock: Boolean get() = (value and desktop_macos_h.NativeCapsLockModifier()) != 0
    public val shift: Boolean get() = (value and desktop_macos_h.NativeShiftModifier()) != 0
    public val control: Boolean get() = (value and desktop_macos_h.NativeControlModifier()) != 0
    public val option: Boolean get() = (value and desktop_macos_h.NativeOptionModifier()) != 0
    public val command: Boolean get() = (value and desktop_macos_h.NativeCommandModifier()) != 0
    public val numericPad: Boolean get() = (value and desktop_macos_h.NativeNumericPadModifier()) != 0
    public val help: Boolean get() = (value and desktop_macos_h.NativeHelpModifier()) != 0
    public val function: Boolean get() = (value and desktop_macos_h.NativeFunctionModifier()) != 0

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

@Suppress("ConstPropertyName")
public object CodepointConstants {
    public const val EnterCharacter: Int = 0x0003
    public const val BackspaceCharacter: Int = 0x0008
    public const val TabCharacter: Int = 0x0009
    public const val NewlineCharacter: Int = 0x000a
    public const val FormFeedCharacter: Int = 0x000c
    public const val CarriageReturnCharacter: Int = 0x000d
    public const val BackTabCharacter: Int = 0x0019
    public const val DeleteCharacter: Int = 0x007f
    public const val LineSeparatorCharacter: Int = 0x2028
    public const val ParagraphSeparatorCharacter: Int = 0x2029

    public const val UpArrowFunctionKey: Int = 0xF700
    public const val DownArrowFunctionKey: Int = 0xF701
    public const val LeftArrowFunctionKey: Int = 0xF702
    public const val RightArrowFunctionKey: Int = 0xF703
    public const val F1FunctionKey: Int = 0xF704
    public const val F2FunctionKey: Int = 0xF705
    public const val F3FunctionKey: Int = 0xF706
    public const val F4FunctionKey: Int = 0xF707
    public const val F5FunctionKey: Int = 0xF708
    public const val F6FunctionKey: Int = 0xF709
    public const val F7FunctionKey: Int = 0xF70A
    public const val F8FunctionKey: Int = 0xF70B
    public const val F9FunctionKey: Int = 0xF70C
    public const val F10FunctionKey: Int = 0xF70D
    public const val F11FunctionKey: Int = 0xF70E
    public const val F12FunctionKey: Int = 0xF70F
    public const val F13FunctionKey: Int = 0xF710
    public const val F14FunctionKey: Int = 0xF711
    public const val F15FunctionKey: Int = 0xF712
    public const val F16FunctionKey: Int = 0xF713
    public const val F17FunctionKey: Int = 0xF714
    public const val F18FunctionKey: Int = 0xF715
    public const val F19FunctionKey: Int = 0xF716
    public const val F20FunctionKey: Int = 0xF717
    public const val F21FunctionKey: Int = 0xF718
    public const val F22FunctionKey: Int = 0xF719
    public const val F23FunctionKey: Int = 0xF71A
    public const val F24FunctionKey: Int = 0xF71B
    public const val F25FunctionKey: Int = 0xF71C
    public const val F26FunctionKey: Int = 0xF71D
    public const val F27FunctionKey: Int = 0xF71E
    public const val F28FunctionKey: Int = 0xF71F
    public const val F29FunctionKey: Int = 0xF720
    public const val F30FunctionKey: Int = 0xF721
    public const val F31FunctionKey: Int = 0xF722
    public const val F32FunctionKey: Int = 0xF723
    public const val F33FunctionKey: Int = 0xF724
    public const val F34FunctionKey: Int = 0xF725
    public const val F35FunctionKey: Int = 0xF726
    public const val InsertFunctionKey: Int = 0xF727
    public const val DeleteFunctionKey: Int = 0xF728
    public const val HomeFunctionKey: Int = 0xF729
    public const val BeginFunctionKey: Int = 0xF72A
    public const val EndFunctionKey: Int = 0xF72B
    public const val PageUpFunctionKey: Int = 0xF72C
    public const val PageDownFunctionKey: Int = 0xF72D
    public const val PrintScreenFunctionKey: Int = 0xF72E
    public const val ScrollLockFunctionKey: Int = 0xF72F
    public const val PauseFunctionKey: Int = 0xF730
    public const val SysReqFunctionKey: Int = 0xF731
    public const val BreakFunctionKey: Int = 0xF732
    public const val ResetFunctionKey: Int = 0xF733
    public const val StopFunctionKey: Int = 0xF734
    public const val MenuFunctionKey: Int = 0xF735
    public const val UserFunctionKey: Int = 0xF736
    public const val SystemFunctionKey: Int = 0xF737
    public const val PrintFunctionKey: Int = 0xF738
    public const val ClearLineFunctionKey: Int = 0xF739
    public const val ClearDisplayFunctionKey: Int = 0xF73A
    public const val InsertLineFunctionKey: Int = 0xF73B
    public const val DeleteLineFunctionKey: Int = 0xF73C
    public const val InsertCharFunctionKey: Int = 0xF73D
    public const val DeleteCharFunctionKey: Int = 0xF73E
    public const val PrevFunctionKey: Int = 0xF73F
    public const val NextFunctionKey: Int = 0xF740
    public const val SelectFunctionKey: Int = 0xF741
    public const val ExecuteFunctionKey: Int = 0xF742
    public const val UndoFunctionKey: Int = 0xF743
    public const val RedoFunctionKey: Int = 0xF744
    public const val FindFunctionKey: Int = 0xF745
    public const val HelpFunctionKey: Int = 0xF746
    public const val ModeSwitchFunctionKey: Int = 0xF747
}
