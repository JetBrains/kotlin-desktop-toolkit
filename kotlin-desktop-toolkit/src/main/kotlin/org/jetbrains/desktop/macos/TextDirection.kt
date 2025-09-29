package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

public enum class TextDirection {
    LeftToRight,
    RightToLeft,
    ;

    internal companion object {
        internal fun fromNative(value: Int): TextDirection {
            return when (value) {
                desktop_macos_h.NativeTextDirection_LeftToRight() -> LeftToRight
                desktop_macos_h.NativeTextDirection_RightToLeft() -> RightToLeft
                else -> throw Error("Unexpected variant $value")
            }
        }
    }

    internal fun toNative(): Int {
        return when (this) {
            LeftToRight -> desktop_macos_h.NativeTextDirection_LeftToRight()
            RightToLeft -> desktop_macos_h.NativeTextDirection_RightToLeft()
        }
    }
}
