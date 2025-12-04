package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h

public enum class Appearance {
    Light,
    Dark,
    ;

    public companion object {
        public fun getCurrent(): Appearance {
            val nativeAppearance = ffiDownCall {
                desktop_win32_h.application_get_appearance()
            }
            return fromNative(nativeAppearance)
        }

        internal fun fromNative(value: Int): Appearance {
            return when (value) {
                desktop_win32_h.NativeAppearance_Light() -> Light
                desktop_win32_h.NativeAppearance_Dark() -> Dark
                else -> throw Error("Unexpected variant $value")
            }
        }
    }

    internal fun toNative(): Int {
        return when (this) {
            Light -> desktop_win32_h.NativeAppearance_Light()
            Dark -> desktop_win32_h.NativeAppearance_Dark()
        }
    }
}
