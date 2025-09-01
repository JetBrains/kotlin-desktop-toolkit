package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h

@JvmInline
public value class MouseKeyState internal constructor(private val value: Short) {
    public companion object {
        public val None: MouseKeyState = MouseKeyState(0)
        public val Control: MouseKeyState = MouseKeyState(0x0008)
        public val LeftButton: MouseKeyState = MouseKeyState(0x0001)
        public val RightButton: MouseKeyState = MouseKeyState(0x0002)
        public val MiddleButton: MouseKeyState = MouseKeyState(0x0010)
        public val Shift: MouseKeyState = MouseKeyState(0x0004)
        public val XButton1: MouseKeyState = MouseKeyState(0x0020)
        public val XButton2: MouseKeyState = MouseKeyState(0x0040)
    }

    public fun hasFlag(keyState: MouseKeyState): Boolean {
        return (this.value.toInt() and keyState.value.toInt()) == keyState.value.toInt()
    }
}

public enum class MouseButton {
    NONE,
    LEFT,
    RIGHT,
    MIDDLE,
    XBUTTON1,
    XBUTTON2,
    ;

    public companion object
}

internal fun MouseKeyState.Companion.fromNative(x: Short): MouseKeyState = MouseKeyState(x)

internal fun MouseButton.Companion.fromNative(x: Int): MouseButton = when (x) {
    desktop_win32_h.NativeMouseButton_None() -> MouseButton.NONE
    desktop_win32_h.NativeMouseButton_Left() -> MouseButton.LEFT
    desktop_win32_h.NativeMouseButton_Right() -> MouseButton.RIGHT
    desktop_win32_h.NativeMouseButton_Middle() -> MouseButton.MIDDLE
    desktop_win32_h.NativeMouseButton_XButton1() -> MouseButton.XBUTTON1
    desktop_win32_h.NativeMouseButton_XButton2() -> MouseButton.XBUTTON2
    else -> error("Unknown mouse button: $x")
}
