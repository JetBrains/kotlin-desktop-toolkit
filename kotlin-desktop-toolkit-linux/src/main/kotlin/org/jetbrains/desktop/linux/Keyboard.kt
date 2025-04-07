package org.jetbrains.desktop.linux

@JvmInline
public value class KeyCode internal constructor(private val value: String) {
    public companion object {
        internal fun fromNative(code: Short): KeyCode {
            return when (code.toInt()) {
                else -> KeyCode("") // TODO
            }
        }
    }
}

// TODO
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
