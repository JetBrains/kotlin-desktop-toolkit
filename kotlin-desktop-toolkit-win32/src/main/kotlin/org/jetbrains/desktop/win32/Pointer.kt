package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativePointerState
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.MemorySegment

public data class PointerState(val pressedButtons: PointerButtons) {
    internal companion object
}

@JvmInline
public value class PointerButton internal constructor(internal val value: Int) {
    public companion object {
        public val None: PointerButton = PointerButton(0)
        public val LeftButton: PointerButton = PointerButton(1)
        public val RightButton: PointerButton = PointerButton(2)
        public val MiddleButton: PointerButton = PointerButton(4)
        public val XButton1: PointerButton = PointerButton(8)
        public val XButton2: PointerButton = PointerButton(16)
    }
}

@JvmInline
public value class PointerButtons internal constructor(private val value: Int) {
    public fun hasFlag(button: PointerButton): Boolean {
        return (this.value and button.value) == button.value
    }
}

internal fun PointerState.Companion.fromNative(s: MemorySegment): PointerState {
    return PointerState(
        pressedButtons = PointerButtons(NativePointerState.pressed_buttons(s)),
    )
}

internal fun PointerButton.Companion.fromNative(x: Int): PointerButton = when (x) {
    desktop_win32_h.NativePointerButton_None() -> None
    desktop_win32_h.NativePointerButton_Left() -> LeftButton
    desktop_win32_h.NativePointerButton_Right() -> RightButton
    desktop_win32_h.NativePointerButton_Middle() -> MiddleButton
    desktop_win32_h.NativePointerButton_XButton1() -> XButton1
    desktop_win32_h.NativePointerButton_XButton2() -> XButton2
    else -> error("Unknown pointer button: $x")
}
