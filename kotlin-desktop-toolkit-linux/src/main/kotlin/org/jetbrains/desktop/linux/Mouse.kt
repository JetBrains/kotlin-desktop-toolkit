package org.jetbrains.desktop.linux

@JvmInline
public value class MouseButton internal constructor(public val value: Int) {
    /* From linux/input-event-codes.h - the buttons usually used by mice */
    public companion object {
        public val LEFT: MouseButton = MouseButton(0x110)

        public val RIGHT: MouseButton = MouseButton(0x111)

        public val MIDDLE: MouseButton = MouseButton(0x112)

        /** The fourth non-scroll button, which is often used as "back" in web browsers. */
        public val SIDE: MouseButton = MouseButton(0x113)

        /** The fifth non-scroll button, which is often used as "forward" in web browsers. */
        public val EXTRA: MouseButton = MouseButton(0x114)

        /** @see [EXTRA] */
        public val FORWARD: MouseButton = MouseButton(0x115)

        /** @see [SIDE] */
        public val BACK: MouseButton = MouseButton(0x116)

        public val TASK: MouseButton = MouseButton(0x117)
    }

    override fun toString(): String {
        return when (this) {
            LEFT -> "MouseButton.LEFT"
            RIGHT -> "MouseButton.RIGHT"
            MIDDLE -> "MouseButton.MIDDLE"
            SIDE -> "MouseButton.MIDDLE"
            EXTRA -> "MouseButton.EXTRA"
            FORWARD -> "MouseButton.FORWARD"
            BACK -> "MouseButton.BACK"
            TASK -> "MouseButton.TASK"
            else -> "MouseButton.Other($value)"
        }
    }
}

@JvmInline
public value class MouseButtonsSet internal constructor(private val value: Int) : Iterable<MouseButton> {
    public fun contains(button: MouseButton): Boolean {
        return 1.shl(button.value).and(value) != 0
    }

    private fun toList(): List<MouseButton> {
        return IntRange(0, Int.SIZE_BITS - 1).mapNotNull { i ->
            val button = MouseButton(i)
            if (contains(button)) {
                button
            } else {
                null
            }
        }
    }

    override fun iterator(): Iterator<MouseButton> {
        return toList().iterator()
    }

    override fun toString(): String {
        return toList().toString()
    }
}
