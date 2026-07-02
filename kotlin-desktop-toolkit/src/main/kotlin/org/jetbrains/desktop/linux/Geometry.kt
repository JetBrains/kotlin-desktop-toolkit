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
    val width: Int,
    val height: Int,
) {
    init {
        require(width >= 0 && height >= 0) {
            "Invalid size (both width and height must be positive)"
        }
    }

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
    val x: Int,
    val y: Int,
    val width: Int,
    val height: Int,
)

public data class LogicalSideOffsets(
    val top: Int,
    val left: Int,
    val bottom: Int,
    val right: Int,
) {
    public constructor(all: Int) : this(all, all, all, all)

    init {
        require(top >= 0 && left >= 0 && bottom >= 0 && right >= 0) {
            "Invalid SideOffsets (top, left, bottom and right must be positive)"
        }
    }

    public companion object {
        public val Zero: LogicalSideOffsets = LogicalSideOffsets(0)
    }

    public fun toPhysical(scale: Double): PhysicalSideOffsets = PhysicalSideOffsets(
        top = (top.toDouble() * scale).roundToInt(),
        left = (left.toDouble() * scale).roundToInt(),
        bottom = (bottom.toDouble() * scale).roundToInt(),
        right = (right.toDouble() * scale).roundToInt(),
    )
}

public data class PhysicalSideOffsets(
    val top: PhysicalPixels,
    val left: PhysicalPixels,
    val bottom: PhysicalPixels,
    val right: PhysicalPixels,
) {
    public constructor(all: Int) : this(all, all, all, all)

    public companion object {
        public val Zero: PhysicalSideOffsets = PhysicalSideOffsets(0)
    }
}
