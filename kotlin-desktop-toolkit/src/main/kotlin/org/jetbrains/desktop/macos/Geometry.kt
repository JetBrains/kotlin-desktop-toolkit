package org.jetbrains.desktop.macos

public typealias PhysicalPixels = Double
public typealias LogicalPixels = Double

public data class PhysicalSize(
    val width: PhysicalPixels,
    val height: PhysicalPixels,
) {
    public companion object {}
    public fun toLogical(scale: Double): LogicalSize = LogicalSize(width / scale, height / scale)
}

public data class PhysicalPoint(
    val x: PhysicalPixels,
    val y: PhysicalPixels,
) {
    public companion object {}
    public fun toLogical(scale: Double): LogicalPoint = LogicalPoint(x / scale, y / scale)
}

public data class LogicalSize(
    val width: LogicalPixels,
    val height: LogicalPixels,
) {
    public companion object {}
    public fun toPhysical(scale: Double): PhysicalSize = PhysicalSize(width * scale, height * scale)
}

public data class LogicalPoint(
    val x: LogicalPixels,
    val y: LogicalPixels,
) {
    public companion object {
        public val Zero: LogicalPoint = LogicalPoint(0.0, 0.0)
    }
    public fun toPhysical(scale: Double): PhysicalPoint = PhysicalPoint(x * scale, y * scale)
}
