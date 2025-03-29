package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h

public enum class Appearance {
    Light,
    Dark,
    ;

    internal companion object {
        internal fun fromNative(value: Int): Appearance {
            return when (value) {
                desktop_macos_h.NativeAppearance_Light() -> Light
                desktop_macos_h.NativeAppearance_Dark() -> Dark
                else -> throw Error("Unexpected variant $value")
            }
        }
    }

    internal fun toNative(): Int {
        return when (this) {
            Light -> desktop_macos_h.NativeAppearance_Light()
            Dark -> desktop_macos_h.NativeAppearance_Dark()
        }
    }
}
