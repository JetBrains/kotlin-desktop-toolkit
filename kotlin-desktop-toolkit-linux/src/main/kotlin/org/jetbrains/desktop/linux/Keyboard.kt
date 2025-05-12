package org.jetbrains.desktop.linux

@JvmInline
public value class KeyCode internal constructor(public val value: Int)

public data class KeyModifiers(
    val capsLock: Boolean,
    val shift: Boolean,
    val control: Boolean,
    val alt: Boolean,
    val logo: Boolean,
    val numLock: Boolean,
) {
    internal companion object
}

public typealias RawKeysym = Int

@JvmInline
public value class KeySym internal constructor(public val value: Int) {
    public fun isModifierKey(): Boolean {
        return (
            value in Shift_L..Hyper_R ||
                value in ISO_Lock..ISO_Level5_Lock ||
                value == Mode_switch ||
                value == Num_Lock
            )
    }

    @Suppress("MemberVisibilityCanBePrivate", "ConstPropertyName", "ktlint:standard:property-naming")
    public companion object {
        public const val BackSpace: RawKeysym = 0xff08
        public const val Tab: RawKeysym = 0xff09
        public const val Linefeed: RawKeysym = 0xff0a
        public const val Clear: RawKeysym = 0xff0b
        public const val Return: RawKeysym = 0xff0d
        public const val Pause: RawKeysym = 0xff13
        public const val Scroll_Lock: RawKeysym = 0xff14
        public const val Sys_Req: RawKeysym = 0xff15
        public const val Escape: RawKeysym = 0xff1b
        public const val Delete: RawKeysym = 0xffff
        public const val Multi_key: RawKeysym = 0xff20
        public const val Codeinput: RawKeysym = 0xff37
        public const val SingleCandidate: RawKeysym = 0xff3c
        public const val MultipleCandidate: RawKeysym = 0xff3d
        public const val PreviousCandidate: RawKeysym = 0xff3e
        public const val Kanji: RawKeysym = 0xff21
        public const val Muhenkan: RawKeysym = 0xff22
        public const val Henkan_Mode: RawKeysym = 0xff23
        public const val Henkan: RawKeysym = 0xff23
        public const val Romaji: RawKeysym = 0xff24
        public const val Hiragana: RawKeysym = 0xff25
        public const val Katakana: RawKeysym = 0xff26
        public const val Hiragana_Katakana: RawKeysym = 0xff27
        public const val Zenkaku: RawKeysym = 0xff28
        public const val Hankaku: RawKeysym = 0xff29
        public const val Zenkaku_Hankaku: RawKeysym = 0xff2a
        public const val Touroku: RawKeysym = 0xff2b
        public const val Massyo: RawKeysym = 0xff2c
        public const val Kana_Lock: RawKeysym = 0xff2d
        public const val Kana_Shift: RawKeysym = 0xff2e
        public const val Eisu_Shift: RawKeysym = 0xff2f
        public const val Eisu_toggle: RawKeysym = 0xff30
        public const val Kanji_Bangou: RawKeysym = 0xff37
        public const val Zen_Koho: RawKeysym = 0xff3d
        public const val Mae_Koho: RawKeysym = 0xff3e
        public const val Home: RawKeysym = 0xff50
        public const val Left: RawKeysym = 0xff51
        public const val Up: RawKeysym = 0xff52
        public const val Right: RawKeysym = 0xff53
        public const val Down: RawKeysym = 0xff54
        public const val Prior: RawKeysym = 0xff55
        public const val Page_Up: RawKeysym = 0xff55
        public const val Next: RawKeysym = 0xff56
        public const val Page_Down: RawKeysym = 0xff56
        public const val End: RawKeysym = 0xff57
        public const val Begin: RawKeysym = 0xff58
        public const val Select: RawKeysym = 0xff60
        public const val Print: RawKeysym = 0xff61
        public const val Execute: RawKeysym = 0xff62
        public const val Insert: RawKeysym = 0xff63
        public const val Undo: RawKeysym = 0xff65
        public const val Redo: RawKeysym = 0xff66
        public const val Menu: RawKeysym = 0xff67
        public const val Find: RawKeysym = 0xff68
        public const val Cancel: RawKeysym = 0xff69
        public const val Help: RawKeysym = 0xff6a
        public const val Break: RawKeysym = 0xff6b
        public const val Mode_switch: RawKeysym = 0xff7e
        public const val script_switch: RawKeysym = 0xff7e
        public const val Num_Lock: RawKeysym = 0xff7f
        public const val KP_Space: RawKeysym = 0xff80
        public const val KP_Tab: RawKeysym = 0xff89
        public const val KP_Enter: RawKeysym = 0xff8d
        public const val KP_F1: RawKeysym = 0xff91
        public const val KP_F2: RawKeysym = 0xff92
        public const val KP_F3: RawKeysym = 0xff93
        public const val KP_F4: RawKeysym = 0xff94
        public const val KP_Home: RawKeysym = 0xff95
        public const val KP_Left: RawKeysym = 0xff96
        public const val KP_Up: RawKeysym = 0xff97
        public const val KP_Right: RawKeysym = 0xff98
        public const val KP_Down: RawKeysym = 0xff99
        public const val KP_Prior: RawKeysym = 0xff9a
        public const val KP_Page_Up: RawKeysym = 0xff9a
        public const val KP_Next: RawKeysym = 0xff9b
        public const val KP_Page_Down: RawKeysym = 0xff9b
        public const val KP_End: RawKeysym = 0xff9c
        public const val KP_Begin: RawKeysym = 0xff9d
        public const val KP_Insert: RawKeysym = 0xff9e
        public const val KP_Delete: RawKeysym = 0xff9f
        public const val KP_Equal: RawKeysym = 0xffbd
        public const val KP_Multiply: RawKeysym = 0xffaa
        public const val KP_Add: RawKeysym = 0xffab
        public const val KP_Separator: RawKeysym = 0xffac
        public const val KP_Subtract: RawKeysym = 0xffad
        public const val KP_Decimal: RawKeysym = 0xffae
        public const val KP_Divide: RawKeysym = 0xffaf
        public const val KP_0: RawKeysym = 0xffb0
        public const val KP_1: RawKeysym = 0xffb1
        public const val KP_2: RawKeysym = 0xffb2
        public const val KP_3: RawKeysym = 0xffb3
        public const val KP_4: RawKeysym = 0xffb4
        public const val KP_5: RawKeysym = 0xffb5
        public const val KP_6: RawKeysym = 0xffb6
        public const val KP_7: RawKeysym = 0xffb7
        public const val KP_8: RawKeysym = 0xffb8
        public const val KP_9: RawKeysym = 0xffb9
        public const val F1: RawKeysym = 0xffbe
        public const val F2: RawKeysym = 0xffbf
        public const val F3: RawKeysym = 0xffc0
        public const val F4: RawKeysym = 0xffc1
        public const val F5: RawKeysym = 0xffc2
        public const val F6: RawKeysym = 0xffc3
        public const val F7: RawKeysym = 0xffc4
        public const val F8: RawKeysym = 0xffc5
        public const val F9: RawKeysym = 0xffc6
        public const val F10: RawKeysym = 0xffc7
        public const val F11: RawKeysym = 0xffc8
        public const val L1: RawKeysym = 0xffc8
        public const val F12: RawKeysym = 0xffc9
        public const val L2: RawKeysym = 0xffc9
        public const val F13: RawKeysym = 0xffca
        public const val L3: RawKeysym = 0xffca
        public const val F14: RawKeysym = 0xffcb
        public const val L4: RawKeysym = 0xffcb
        public const val F15: RawKeysym = 0xffcc
        public const val L5: RawKeysym = 0xffcc
        public const val F16: RawKeysym = 0xffcd
        public const val L6: RawKeysym = 0xffcd
        public const val F17: RawKeysym = 0xffce
        public const val L7: RawKeysym = 0xffce
        public const val F18: RawKeysym = 0xffcf
        public const val L8: RawKeysym = 0xffcf
        public const val F19: RawKeysym = 0xffd0
        public const val L9: RawKeysym = 0xffd0
        public const val F20: RawKeysym = 0xffd1
        public const val L10: RawKeysym = 0xffd1
        public const val F21: RawKeysym = 0xffd2
        public const val R1: RawKeysym = 0xffd2
        public const val F22: RawKeysym = 0xffd3
        public const val R2: RawKeysym = 0xffd3
        public const val F23: RawKeysym = 0xffd4
        public const val R3: RawKeysym = 0xffd4
        public const val F24: RawKeysym = 0xffd5
        public const val R4: RawKeysym = 0xffd5
        public const val F25: RawKeysym = 0xffd6
        public const val R5: RawKeysym = 0xffd6
        public const val F26: RawKeysym = 0xffd7
        public const val R6: RawKeysym = 0xffd7
        public const val F27: RawKeysym = 0xffd8
        public const val R7: RawKeysym = 0xffd8
        public const val F28: RawKeysym = 0xffd9
        public const val R8: RawKeysym = 0xffd9
        public const val F29: RawKeysym = 0xffda
        public const val R9: RawKeysym = 0xffda
        public const val F30: RawKeysym = 0xffdb
        public const val R10: RawKeysym = 0xffdb
        public const val F31: RawKeysym = 0xffdc
        public const val R11: RawKeysym = 0xffdc
        public const val F32: RawKeysym = 0xffdd
        public const val R12: RawKeysym = 0xffdd
        public const val F33: RawKeysym = 0xffde
        public const val R13: RawKeysym = 0xffde
        public const val F34: RawKeysym = 0xffdf
        public const val R14: RawKeysym = 0xffdf
        public const val F35: RawKeysym = 0xffe0
        public const val R15: RawKeysym = 0xffe0
        public const val Shift_L: RawKeysym = 0xffe1
        public const val Shift_R: RawKeysym = 0xffe2
        public const val Control_L: RawKeysym = 0xffe3
        public const val Control_R: RawKeysym = 0xffe4
        public const val Caps_Lock: RawKeysym = 0xffe5
        public const val Shift_Lock: RawKeysym = 0xffe6
        public const val Meta_L: RawKeysym = 0xffe7
        public const val Meta_R: RawKeysym = 0xffe8
        public const val Alt_L: RawKeysym = 0xffe9
        public const val Alt_R: RawKeysym = 0xffea
        public const val Super_L: RawKeysym = 0xffeb
        public const val Super_R: RawKeysym = 0xffec
        public const val Hyper_L: RawKeysym = 0xffed
        public const val Hyper_R: RawKeysym = 0xffee
        public const val ISO_Lock: RawKeysym = 0xfe01
        public const val ISO_Level2_Latch: RawKeysym = 0xfe02
        public const val ISO_Level3_Shift: RawKeysym = 0xfe03
        public const val ISO_Level3_Latch: RawKeysym = 0xfe04
        public const val ISO_Level3_Lock: RawKeysym = 0xfe05
        public const val ISO_Level5_Shift: RawKeysym = 0xfe11
        public const val ISO_Level5_Latch: RawKeysym = 0xfe12
        public const val ISO_Level5_Lock: RawKeysym = 0xfe13
        public const val ISO_Enter: RawKeysym = 0xfe34
        public const val space: RawKeysym = 0x20
        public const val exclam: RawKeysym = 0x21
        public const val quotedbl: RawKeysym = 0x22
        public const val numbersign: RawKeysym = 0x23
        public const val dollar: RawKeysym = 0x24
        public const val percent: RawKeysym = 0x25
        public const val ampersand: RawKeysym = 0x26
        public const val apostrophe: RawKeysym = 0x27
        public const val quoteright: RawKeysym = 0x27
        public const val parenleft: RawKeysym = 0x28
        public const val parenright: RawKeysym = 0x29
        public const val asterisk: RawKeysym = 0x2a
        public const val plus: RawKeysym = 0x2b
        public const val comma: RawKeysym = 0x2c
        public const val minus: RawKeysym = 0x2d
        public const val period: RawKeysym = 0x2e
        public const val slash: RawKeysym = 0x2f
        public const val _0: RawKeysym = 0x30
        public const val _1: RawKeysym = 0x31
        public const val _2: RawKeysym = 0x32
        public const val _3: RawKeysym = 0x33
        public const val _4: RawKeysym = 0x34
        public const val _5: RawKeysym = 0x35
        public const val _6: RawKeysym = 0x36
        public const val _7: RawKeysym = 0x37
        public const val _8: RawKeysym = 0x38
        public const val _9: RawKeysym = 0x39
        public const val colon: RawKeysym = 0x3a
        public const val semicolon: RawKeysym = 0x3b
        public const val less: RawKeysym = 0x3c
        public const val equal: RawKeysym = 0x3d
        public const val greater: RawKeysym = 0x3e
        public const val question: RawKeysym = 0x3f
        public const val at: RawKeysym = 0x40
        public const val A: RawKeysym = 0x41
        public const val B: RawKeysym = 0x42
        public const val C: RawKeysym = 0x43
        public const val D: RawKeysym = 0x44
        public const val E: RawKeysym = 0x45
        public const val F: RawKeysym = 0x46
        public const val G: RawKeysym = 0x47
        public const val H: RawKeysym = 0x48
        public const val I: RawKeysym = 0x49
        public const val J: RawKeysym = 0x4a
        public const val K: RawKeysym = 0x4b
        public const val L: RawKeysym = 0x4c
        public const val M: RawKeysym = 0x4d
        public const val N: RawKeysym = 0x4e
        public const val O: RawKeysym = 0x4f
        public const val P: RawKeysym = 0x50
        public const val Q: RawKeysym = 0x51
        public const val R: RawKeysym = 0x52
        public const val S: RawKeysym = 0x53
        public const val T: RawKeysym = 0x54
        public const val U: RawKeysym = 0x55
        public const val V: RawKeysym = 0x56
        public const val W: RawKeysym = 0x57
        public const val X: RawKeysym = 0x58
        public const val Y: RawKeysym = 0x59
        public const val Z: RawKeysym = 0x5a
        public const val bracketleft: RawKeysym = 0x5b
        public const val backslash: RawKeysym = 0x5c
        public const val bracketright: RawKeysym = 0x5d
        public const val asciicircum: RawKeysym = 0x5e
        public const val underscore: RawKeysym = 0x5f
        public const val grave: RawKeysym = 0x60
        public const val quoteleft: RawKeysym = 0x60
        public const val a: RawKeysym = 0x61
        public const val b: RawKeysym = 0x62
        public const val c: RawKeysym = 0x63
        public const val d: RawKeysym = 0x64
        public const val e: RawKeysym = 0x65
        public const val f: RawKeysym = 0x66
        public const val g: RawKeysym = 0x67
        public const val h: RawKeysym = 0x68
        public const val i: RawKeysym = 0x69
        public const val j: RawKeysym = 0x6a
        public const val k: RawKeysym = 0x6b
        public const val l: RawKeysym = 0x6c
        public const val m: RawKeysym = 0x6d
        public const val n: RawKeysym = 0x6e
        public const val o: RawKeysym = 0x6f
        public const val p: RawKeysym = 0x70
        public const val q: RawKeysym = 0x71
        public const val r: RawKeysym = 0x72
        public const val s: RawKeysym = 0x73
        public const val t: RawKeysym = 0x74
        public const val u: RawKeysym = 0x75
        public const val v: RawKeysym = 0x76
        public const val w: RawKeysym = 0x77
        public const val x: RawKeysym = 0x78
        public const val y: RawKeysym = 0x79
        public const val z: RawKeysym = 0x7a
        public const val braceleft: RawKeysym = 0x7b
        public const val bar: RawKeysym = 0x7c
        public const val braceright: RawKeysym = 0x7d
    }
}
