package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

@JvmInline
value class MouseButton internal constructor(val value: Int) {
    companion object {
        val LEFT = MouseButton(desktop_macos_h.LeftMouseButton())
        val RIGHT = MouseButton(desktop_macos_h.RightMouseButton())
        val MIDDLE = MouseButton(desktop_macos_h.MiddleMouseButton())
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
value class MouseButtonsSet internal constructor(val value: Int) : Iterable<MouseButton> {
    fun contains(button: MouseButton): Boolean {
        return 1.shl(button.value).and(value) != 0
    }

    fun toList(): List<MouseButton> {
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
