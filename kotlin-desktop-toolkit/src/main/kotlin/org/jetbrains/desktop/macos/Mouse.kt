package org.jetbrains.desktop.macos

@JvmInline
public value class MouseButton internal constructor(public val value: Int) {
    public companion object {
        public val LEFT: MouseButton = MouseButton(0)
        public val RIGHT: MouseButton = MouseButton(1)
        public val MIDDLE: MouseButton = MouseButton(2)
    }

    override fun toString(): String {
        return when (this) {
            LEFT -> "MouseButton.LEFT"
            RIGHT -> "MouseButton.RIGHT"
            MIDDLE -> "MouseButton.MIDDLE"
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
