package org.jetbrains.desktop.win32

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
)

public data class LogicalPoint(
    val x: LogicalPixels,
    val y: LogicalPixels,
) {
    public companion object {
        public val Zero: LogicalPoint = LogicalPoint(0f, 0f)
    }
}
