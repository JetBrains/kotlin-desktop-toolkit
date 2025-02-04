package org.jetbrains.desktop

typealias PhysicalPixels = Double
typealias LogicalPixels = Double

data class PhysicalSize(val width: PhysicalPixels, val height: PhysicalPixels) {
    companion object

    fun toLogical(scale: Double) = LogicalSize(width / scale, height / scale)
}

data class PhysicalPoint(val x: PhysicalPixels, val y: PhysicalPixels) {
    companion object

    fun toLogical(scale: Double) = LogicalPoint(x / scale, y / scale)
}

data class LogicalSize(val width: LogicalPixels, val height: LogicalPixels) {
    companion object

    fun toPhysical(scale: Double) = PhysicalSize(width * scale, height * scale)
}

data class LogicalPoint(val x: LogicalPixels, val y: LogicalPixels) {
    companion object {
        val Zero = LogicalPoint(0.0, 0.0)
    }

    fun toPhysical(scale: Double) = PhysicalPoint(x * scale, y * scale)
}