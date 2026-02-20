package org.jetbrains.desktop.gtk

import kotlin.math.roundToInt

public typealias PhysicalPixels = Int
public typealias LogicalPixels = Float

public data class PhysicalSize(
    val width: PhysicalPixels,
    val height: PhysicalPixels,
) {
    public companion object;
}

public data class PhysicalPoint(
    val x: PhysicalPixels,
    val y: PhysicalPixels,
) {
    public companion object;
}

public data class LogicalSize(
    val width: Int,
    val height: Int,
) {
    init {
        check(width >= 0 && height >= 0) {
            "Invalid size (both width and height must be positive)"
        }
    }

    public companion object;
    public fun toPhysical(scale: Float): PhysicalSize =
        PhysicalSize((width.toFloat() * scale).roundToInt(), (height.toFloat() * scale).roundToInt())
}

public data class LogicalPoint(
    val x: LogicalPixels,
    val y: LogicalPixels,
) {
    public companion object {
        public val Zero: LogicalPoint = LogicalPoint(0f, 0f)
    }
    public fun toPhysical(scale: Float): PhysicalPoint = PhysicalPoint((x * scale).roundToInt(), (y * scale).roundToInt())
}

public data class LogicalRect(
    val x: Int,
    val y: Int,
    val width: Int,
    val height: Int,
) {
    public fun contains(p: LogicalPoint): Boolean {
        return p.x > x.toFloat() &&
            p.x < (x + width).toFloat() &&
            p.y > y.toFloat() &&
            p.y < (y + height).toFloat()
    }
}
