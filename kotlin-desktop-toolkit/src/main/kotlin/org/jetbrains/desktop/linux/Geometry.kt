package org.jetbrains.desktop.linux

import kotlin.math.roundToInt

public typealias PhysicalPixels = Int
public typealias LogicalPixels = Double

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
    val width: UInt,
    val height: UInt,
) {
    public companion object;
    public fun toPhysical(scale: Double): PhysicalSize {
        val width = (width.toDouble() * scale).roundToInt()
        val height = (height.toDouble() * scale).roundToInt()
        return PhysicalSize(width, height)
    }
}

public data class LogicalPoint(
    val x: LogicalPixels,
    val y: LogicalPixels,
) {
    public companion object {
        public val Zero: LogicalPoint = LogicalPoint(0.0, 0.0)
    }
    public fun toPhysical(scale: Double): PhysicalPoint = PhysicalPoint((x * scale).roundToInt(), (y * scale).roundToInt())
}

public data class LogicalRect(
    val x: UInt,
    val y: UInt,
    val width: UInt,
    val height: UInt,
)
