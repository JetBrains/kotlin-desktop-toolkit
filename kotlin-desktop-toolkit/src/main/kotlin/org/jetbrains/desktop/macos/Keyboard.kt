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
value class KeyCode internal constructor(val value: String) {
    companion object {
        val ANSI_A = KeyCode("ANSI_A")
        val ANSI_S = KeyCode("ANSI_S")
        val ANSI_D = KeyCode("ANSI_D")
        val ANSI_F = KeyCode("ANSI_F")
        val ANSI_H = KeyCode("ANSI_H")
        val ANSI_G = KeyCode("ANSI_G")
        val ANSI_Z = KeyCode("ANSI_Z")
        val ANSI_X = KeyCode("ANSI_X")
        val ANSI_C = KeyCode("ANSI_C")
        val ANSI_V = KeyCode("ANSI_V")
        val ANSI_B = KeyCode("ANSI_B")
        val ANSI_Q = KeyCode("ANSI_Q")
        val ANSI_W = KeyCode("ANSI_W")
        val ANSI_E = KeyCode("ANSI_E")
        val ANSI_R = KeyCode("ANSI_R")
        val ANSI_Y = KeyCode("ANSI_Y")
        val ANSI_T = KeyCode("ANSI_T")
        val ANSI_1 = KeyCode("ANSI_1")
        val ANSI_2 = KeyCode("ANSI_2")
        val ANSI_3 = KeyCode("ANSI_3")
        val ANSI_4 = KeyCode("ANSI_4")
        val ANSI_6 = KeyCode("ANSI_6")
        val ANSI_5 = KeyCode("ANSI_5")
        val ANSI_Equal = KeyCode("ANSI_Equal")
        val ANSI_9 = KeyCode("ANSI_9")
        val ANSI_7 = KeyCode("ANSI_7")
        val ANSI_Minus = KeyCode("ANSI_Minus")
        val ANSI_8 = KeyCode("ANSI_8")
        val ANSI_0 = KeyCode("ANSI_0")
        val ANSI_RightBracket = KeyCode("ANSI_RightBracket")
        val ANSI_O = KeyCode("ANSI_O")
        val ANSI_U = KeyCode("ANSI_U")
        val ANSI_LeftBracket = KeyCode("ANSI_LeftBracket")
        val ANSI_I = KeyCode("ANSI_I")
        val ANSI_P = KeyCode("ANSI_P")
        val ANSI_L = KeyCode("ANSI_L")
        val ANSI_J = KeyCode("ANSI_J")
        val ANSI_Quote = KeyCode("ANSI_Quote")
        val ANSI_K = KeyCode("ANSI_K")
        val ANSI_Semicolon = KeyCode("ANSI_Semicolon")
        val ANSI_Backslash = KeyCode("ANSI_Backslash")
        val ANSI_Comma = KeyCode("ANSI_Comma")
        val ANSI_Slash = KeyCode("ANSI_Slash")
        val ANSI_N = KeyCode("ANSI_N")
        val ANSI_M = KeyCode("ANSI_M")
        val ANSI_Period = KeyCode("ANSI_Period")
        val ANSI_Grave = KeyCode("ANSI_Grave")
        val ANSI_KeypadDecimal = KeyCode("ANSI_KeypadDecimal")
        val ANSI_KeypadMultiply = KeyCode("ANSI_KeypadMultiply")
        val ANSI_KeypadPlus = KeyCode("ANSI_KeypadPlus")
        val ANSI_KeypadClear = KeyCode("ANSI_KeypadClear")
        val ANSI_KeypadDivide = KeyCode("ANSI_KeypadDivide")
        val ANSI_KeypadEnter = KeyCode("ANSI_KeypadEnter")
        val ANSI_KeypadMinus = KeyCode("ANSI_KeypadMinus")
        val ANSI_KeypadEquals = KeyCode("ANSI_KeypadEquals")
        val ANSI_Keypad0 = KeyCode("ANSI_Keypad0")
        val ANSI_Keypad1 = KeyCode("ANSI_Keypad1")
        val ANSI_Keypad2 = KeyCode("ANSI_Keypad2")
        val ANSI_Keypad3 = KeyCode("ANSI_Keypad3")
        val ANSI_Keypad4 = KeyCode("ANSI_Keypad4")
        val ANSI_Keypad5 = KeyCode("ANSI_Keypad5")
        val ANSI_Keypad6 = KeyCode("ANSI_Keypad6")
        val ANSI_Keypad7 = KeyCode("ANSI_Keypad7")
        val ANSI_Keypad8 = KeyCode("ANSI_Keypad8")
        val ANSI_Keypad9 = KeyCode("ANSI_Keypad9")

        /* keycodes for keys that are independent of keyboard layout*/
        val Return = KeyCode("Return")
        val Tab = KeyCode("Tab")
        val Space = KeyCode("Space")
        val Delete = KeyCode("Delete")
        val Escape = KeyCode("Escape")
        val Command = KeyCode("Command")
        val Shift = KeyCode("Shift")
        val CapsLock = KeyCode("CapsLock")
        val Option = KeyCode("Option")
        val Control = KeyCode("Control")
        val RightCommand = KeyCode("RightCommand")
        val RightShift = KeyCode("RightShift")
        val RightOption = KeyCode("RightOption")
        val RightControl = KeyCode("RightControl")
        val Function = KeyCode("Function")
        val F17 = KeyCode("F17")
        val VolumeUp = KeyCode("VolumeUp")
        val VolumeDown = KeyCode("VolumeDown")
        val Mute = KeyCode("Mute")
        val F18 = KeyCode("F18")
        val F19 = KeyCode("F19")
        val F20 = KeyCode("F20")
        val F5 = KeyCode("F5")
        val F6 = KeyCode("F6")
        val F7 = KeyCode("F7")
        val F3 = KeyCode("F3")
        val F8 = KeyCode("F8")
        val F9 = KeyCode("F9")
        val F11 = KeyCode("F11")
        val F13 = KeyCode("F13")
        val F16 = KeyCode("F16")
        val F14 = KeyCode("F14")
        val F10 = KeyCode("F10")
        val ContextualMenu = KeyCode("ContextualMenu")
        val F12 = KeyCode("F12")
        val F15 = KeyCode("F15")
        val Help = KeyCode("Help")
        val Home = KeyCode("Home")
        val PageUp = KeyCode("PageUp")
        val ForwardDelete = KeyCode("ForwardDelete")
        val F4 = KeyCode("F4")
        val End = KeyCode("End")
        val F2 = KeyCode("F2")
        val PageDown = KeyCode("PageDown")
        val F1 = KeyCode("F1")
        val LeftArrow = KeyCode("LeftArrow")
        val RightArrow = KeyCode("RightArrow")
        val DownArrow = KeyCode("DownArrow")
        val UpArrow = KeyCode("UpArrow")

        /* ISO keyboards only*/
        val ISO_Section = KeyCode("ISO_Section")


        val JIS_Yen = KeyCode("JIS_Yen")
        val JIS_Underscore = KeyCode("JIS_Underscore")
        val JIS_KeypadComma = KeyCode("JIS_KeypadComma")
        val JIS_Eisu = KeyCode("JIS_Eisu")
        val JIS_Kana = KeyCode("JIS_Kana")

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

@JvmInline
value class KeyModifiersSet internal constructor(internal val value: Int) {
    companion object {
        fun create(capsLock: Boolean = false,
                   shift: Boolean = false,
                   control: Boolean = false,
                   option: Boolean = false,
                   command: Boolean = false,
                   numericPad: Boolean = false,
                   help: Boolean = false,
                   function: Boolean = false): KeyModifiersSet {
            var result = 0
            if (capsLock) result = result or desktop_macos_h.CapsLockModifier()
            if (shift) result = result or desktop_macos_h.ShiftModifier()
            if (control) result = result or desktop_macos_h.ControlModifier()
            if (option) result = result or desktop_macos_h.OptionModifier()
            if (command) result = result or desktop_macos_h.CommandModifier()
            if (numericPad) result = result or desktop_macos_h.NumericPadModifier()
            if (help) result = result or desktop_macos_h.HelpModifier()
            if (function) result = result or desktop_macos_h.FunctionModifier()
            return KeyModifiersSet(result)
        }
    }

    val capsLock: Boolean get() = (value and desktop_macos_h.CapsLockModifier()) != 0
    val shift: Boolean get() = (value and desktop_macos_h.ShiftModifier()) != 0
    val control: Boolean get() = (value and desktop_macos_h.ControlModifier()) != 0
    val option: Boolean get() = (value and desktop_macos_h.OptionModifier()) != 0
    val command: Boolean get() = (value and desktop_macos_h.CommandModifier()) != 0
    val numericPad: Boolean get() = (value and desktop_macos_h.NumericPadModifier()) != 0
    val help: Boolean get() = (value and desktop_macos_h.HelpModifier()) != 0
    val function: Boolean get() = (value and desktop_macos_h.FunctionModifier()) != 0

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


object CodepointConstants {
    const val EnterCharacter = 0x0003
    const val BackspaceCharacter = 0x0008
    const val TabCharacter = 0x0009
    const val NewlineCharacter = 0x000a
    const val FormFeedCharacter = 0x000c
    const val CarriageReturnCharacter = 0x000d
    const val BackTabCharacter = 0x0019
    const val DeleteCharacter = 0x007f
    const val LineSeparatorCharacter = 0x2028
    const val ParagraphSeparatorCharacter = 0x2029

    const val UpArrowFunctionKey = 0xF700
    const val DownArrowFunctionKey = 0xF701
    const val LeftArrowFunctionKey = 0xF702
    const val RightArrowFunctionKey = 0xF703
    const val F1FunctionKey = 0xF704
    const val F2FunctionKey = 0xF705
    const val F3FunctionKey = 0xF706
    const val F4FunctionKey = 0xF707
    const val F5FunctionKey = 0xF708
    const val F6FunctionKey = 0xF709
    const val F7FunctionKey = 0xF70A
    const val F8FunctionKey = 0xF70B
    const val F9FunctionKey = 0xF70C
    const val F10FunctionKey = 0xF70D
    const val F11FunctionKey = 0xF70E
    const val F12FunctionKey = 0xF70F
    const val F13FunctionKey = 0xF710
    const val F14FunctionKey = 0xF711
    const val F15FunctionKey = 0xF712
    const val F16FunctionKey = 0xF713
    const val F17FunctionKey = 0xF714
    const val F18FunctionKey = 0xF715
    const val F19FunctionKey = 0xF716
    const val F20FunctionKey = 0xF717
    const val F21FunctionKey = 0xF718
    const val F22FunctionKey = 0xF719
    const val F23FunctionKey = 0xF71A
    const val F24FunctionKey = 0xF71B
    const val F25FunctionKey = 0xF71C
    const val F26FunctionKey = 0xF71D
    const val F27FunctionKey = 0xF71E
    const val F28FunctionKey = 0xF71F
    const val F29FunctionKey = 0xF720
    const val F30FunctionKey = 0xF721
    const val F31FunctionKey = 0xF722
    const val F32FunctionKey = 0xF723
    const val F33FunctionKey = 0xF724
    const val F34FunctionKey = 0xF725
    const val F35FunctionKey = 0xF726
    const val InsertFunctionKey = 0xF727
    const val DeleteFunctionKey = 0xF728
    const val HomeFunctionKey = 0xF729
    const val BeginFunctionKey = 0xF72A
    const val EndFunctionKey = 0xF72B
    const val PageUpFunctionKey = 0xF72C
    const val PageDownFunctionKey = 0xF72D
    const val PrintScreenFunctionKey = 0xF72E
    const val ScrollLockFunctionKey = 0xF72F
    const val PauseFunctionKey = 0xF730
    const val SysReqFunctionKey = 0xF731
    const val BreakFunctionKey = 0xF732
    const val ResetFunctionKey = 0xF733
    const val StopFunctionKey = 0xF734
    const val MenuFunctionKey = 0xF735
    const val UserFunctionKey = 0xF736
    const val SystemFunctionKey = 0xF737
    const val PrintFunctionKey = 0xF738
    const val ClearLineFunctionKey = 0xF739
    const val ClearDisplayFunctionKey = 0xF73A
    const val InsertLineFunctionKey = 0xF73B
    const val DeleteLineFunctionKey = 0xF73C
    const val InsertCharFunctionKey = 0xF73D
    const val DeleteCharFunctionKey = 0xF73E
    const val PrevFunctionKey = 0xF73F
    const val NextFunctionKey = 0xF740
    const val SelectFunctionKey = 0xF741
    const val ExecuteFunctionKey = 0xF742
    const val UndoFunctionKey = 0xF743
    const val RedoFunctionKey = 0xF744
    const val FindFunctionKey = 0xF745
    const val HelpFunctionKey = 0xF746
    const val ModeSwitchFunctionKey = 0xF747
}