package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.kwm_macos_h


@JvmInline
value class MouseButton internal constructor(val value: Int) {
    companion object {
        val LEFT = MouseButton(kwm_macos_h.Left())
        val RIGHT = MouseButton(kwm_macos_h.Right())
        val MIDDLE = MouseButton(kwm_macos_h.Middle())
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
value class MouseButtonsSet internal constructor(val value: Int): Iterable<MouseButton> {
    fun contains(button: MouseButton): Boolean {
        return button.value.and(value) != 0
    }

    fun toList(): List<MouseButton> {
        return IntRange(0, Int.SIZE_BITS - 1).mapNotNull { i ->
            val button = MouseButton(1.shl(i))
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