package org.jetbrains.desktop.linux

import kotlin.math.roundToInt

public typealias PhysicalPixels = Int
public typealias LogicalPixels = Float

public data class PhysicalSize(
    val width: PhysicalPixels,
    val height: PhysicalPixels,
) {
    public companion object;
    public fun toLogical(scale: Float): LogicalSize = LogicalSize(width / scale, height / scale)
}

public data class PhysicalPoint(
    val x: PhysicalPixels,
    val y: PhysicalPixels,
) {
    public companion object;
    public fun toLogical(scale: Float): LogicalPoint = LogicalPoint(x / scale, y / scale)
}

public data class LogicalSize(
    val width: LogicalPixels,
    val height: LogicalPixels,
) {
    public companion object;
    public fun toPhysical(scale: Float): PhysicalSize = PhysicalSize((width * scale).roundToInt(), (height * scale).roundToInt())
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

public data class LogicalRect(val point: LogicalPoint, val size: LogicalSize) {
    public fun contains(p: LogicalPoint): Boolean {
        return p.x > point.x &&
            p.x < point.x + size.width &&
            p.y > point.y &&
            p.y < point.y + size.height
    }
}
