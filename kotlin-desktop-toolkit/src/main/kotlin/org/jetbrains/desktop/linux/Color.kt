package org.jetbrains.desktop.linux

/** RGBA values in the sRGB color space, in the range [0,1] */
public data class Color(
    val red: Double,
    val green: Double,
    val blue: Double,
    val alpha: Double,
) {
    internal companion object;
}
