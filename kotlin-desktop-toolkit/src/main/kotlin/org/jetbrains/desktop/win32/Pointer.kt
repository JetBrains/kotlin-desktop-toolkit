package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativePointerState
import java.lang.foreign.MemorySegment

public data class PointerState(
    val pressedButtons: PointerButtons,
    val modifiers: PointerModifiers,
) {
    internal companion object
}

@JvmInline
public value class PointerButtons internal constructor(private val value: Int) {
    public companion object {
        public val None: PointerButtons = PointerButtons(0)
        public val LeftButton: PointerButtons = PointerButtons(1)
        public val RightButton: PointerButtons = PointerButtons(2)
        public val MiddleButton: PointerButtons = PointerButtons(4)
        public val XButton1: PointerButtons = PointerButtons(8)
        public val XButton2: PointerButtons = PointerButtons(16)
    }

    public fun hasFlag(button: PointerButtons): Boolean {
        return (this.value and button.value) == button.value
    }
}

@JvmInline
public value class PointerModifiers internal constructor(private val value: Int) {
    public companion object {
        public val None: PointerModifiers = PointerModifiers(0)
        public val Shift: PointerModifiers = PointerModifiers(4)
        public val Control: PointerModifiers = PointerModifiers(8)
    }

    public fun hasFlag(button: PointerModifiers): Boolean {
        return (this.value and button.value) == button.value
    }
}

internal fun PointerState.Companion.fromNative(s: MemorySegment): PointerState {
    return PointerState(
        pressedButtons = PointerButtons(NativePointerState.pressed_buttons(s)),
        modifiers = PointerModifiers(NativePointerState.modifiers(s)),
    )
}
