package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h

public enum class FontSmoothing {
    Disabled,
    Enabled,
    ;

    public companion object {
        public fun getCurrent(): FontSmoothing {
            val native = ffiDownCall {
                desktop_win32_h.application_get_font_smoothing()
            }
            return fromNative(native)
        }

        internal fun fromNative(value: Int): FontSmoothing {
            return when (value) {
                desktop_win32_h.NativeFontSmoothing_Disabled() -> Disabled
                desktop_win32_h.NativeFontSmoothing_Enabled() -> Enabled
                else -> throw Error("Unexpected variant $value")
            }
        }
    }

    internal fun toNative(): Int {
        return when (this) {
            Disabled -> desktop_win32_h.NativeFontSmoothing_Disabled()
            Enabled -> desktop_win32_h.NativeFontSmoothing_Enabled()
        }
    }
}

public enum class FontSmoothingType {
    Standard,
    ClearType,
    ;

    public companion object {
        public fun getCurrent(): FontSmoothingType {
            val native = ffiDownCall {
                desktop_win32_h.application_get_font_smoothing_type()
            }
            return fromNative(native)
        }

        internal fun fromNative(value: Int): FontSmoothingType {
            return when (value) {
                desktop_win32_h.NativeFontSmoothingType_Standard() -> Standard
                desktop_win32_h.NativeFontSmoothingType_ClearType() -> ClearType
                else -> throw Error("Unexpected variant $value")
            }
        }
    }

    internal fun toNative(): Int {
        return when (this) {
            Standard -> desktop_win32_h.NativeFontSmoothingType_Standard()
            ClearType -> desktop_win32_h.NativeFontSmoothingType_ClearType()
        }
    }
}

public fun getFontSmoothingContrast(): Int = ffiDownCall {
    desktop_win32_h.application_get_font_smoothing_contrast()
}

public enum class FontSmoothingOrientation {
    Rgb,
    Bgr,
    ;

    public companion object {
        public fun getCurrent(): FontSmoothingOrientation {
            val native = ffiDownCall {
                desktop_win32_h.application_get_font_smoothing_orientation()
            }
            return fromNative(native)
        }

        internal fun fromNative(value: Int): FontSmoothingOrientation {
            return when (value) {
                desktop_win32_h.NativeFontSmoothingOrientation_Rgb() -> Rgb
                desktop_win32_h.NativeFontSmoothingOrientation_Bgr() -> Bgr
                else -> throw Error("Unexpected variant $value")
            }
        }
    }

    internal fun toNative(): Int {
        return when (this) {
            Rgb -> desktop_win32_h.NativeFontSmoothingOrientation_Rgb()
            Bgr -> desktop_win32_h.NativeFontSmoothingOrientation_Bgr()
        }
    }
}
