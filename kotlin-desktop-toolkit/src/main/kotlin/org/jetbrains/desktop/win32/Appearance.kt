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

public enum class HighContrast {
    Off,
    On,
    ;

    public companion object {
        public fun getCurrent(): HighContrast {
            val nativeHighContrast = ffiDownCall {
                desktop_win32_h.application_get_high_contrast()
            }
            return fromNative(nativeHighContrast)
        }

        internal fun fromNative(value: Int): HighContrast {
            return when (value) {
                desktop_win32_h.NativeHighContrast_Off() -> Off
                desktop_win32_h.NativeHighContrast_On() -> On
                else -> throw Error("Unexpected variant $value")
            }
        }
    }

    internal fun toNative(): Int {
        return when (this) {
            Off -> desktop_win32_h.NativeHighContrast_Off()
            On -> desktop_win32_h.NativeHighContrast_On()
        }
    }
}
