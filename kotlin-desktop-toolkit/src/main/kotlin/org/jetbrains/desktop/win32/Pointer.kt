package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativePointerState
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.MemorySegment

public data class PointerState(
    val pressedButtons: PointerButtons,
    val modifiers: PointerModifiers,
) {
    internal companion object
}

@JvmInline
public value class PointerButton private constructor(internal val value: Int) {
    public companion object {
        public val None: PointerButton = PointerButton(desktop_win32_h.NativePointerButton_None())
        public val Left: PointerButton = PointerButton(desktop_win32_h.NativePointerButton_Left())
        public val Right: PointerButton = PointerButton(desktop_win32_h.NativePointerButton_Right())
        public val Middle: PointerButton = PointerButton(desktop_win32_h.NativePointerButton_Middle())
        public val XButton1: PointerButton = PointerButton(desktop_win32_h.NativePointerButton_XButton1())
        public val XButton2: PointerButton = PointerButton(desktop_win32_h.NativePointerButton_XButton2())
    }
}

@JvmInline
public value class PointerButtons internal constructor(private val value: Int) {
    public fun hasFlag(button: PointerButton): Boolean {
        return (this.value and button.value) == button.value
    }
}

@JvmInline
public value class PointerModifier private constructor(internal val value: Int) {
    // See https://learn.microsoft.com/en-us/windows/win32/inputmsg/modifier-key-states-constants
    public companion object {
        public val None: PointerModifier = PointerModifier(0)
        public val Shift: PointerModifier = PointerModifier(4)
        public val Control: PointerModifier = PointerModifier(8)
    }
}

@JvmInline
public value class PointerModifiers internal constructor(private val value: Int) {
    public fun hasFlag(modifier: PointerModifier): Boolean {
        return (this.value and modifier.value) == modifier.value
    }
}

internal fun PointerButton.Companion.fromNative(value: Int): PointerButton {
    // additional validation; for a set of flags [PointerButtons] should be used
    return when (value) {
        desktop_win32_h.NativePointerButton_None() -> PointerButton.None
        desktop_win32_h.NativePointerButton_Left() -> PointerButton.Left
        desktop_win32_h.NativePointerButton_Right() -> PointerButton.Right
        desktop_win32_h.NativePointerButton_Middle() -> PointerButton.Middle
        desktop_win32_h.NativePointerButton_XButton1() -> PointerButton.XButton1
        desktop_win32_h.NativePointerButton_XButton2() -> PointerButton.XButton2
        else -> error("Unknown pointer button value: $value")
    }
}

internal fun PointerState.Companion.fromNative(s: MemorySegment): PointerState {
    return PointerState(
        pressedButtons = PointerButtons(NativePointerState.pressed_buttons(s)),
        modifiers = PointerModifiers(NativePointerState.modifiers(s)),
    )
}
