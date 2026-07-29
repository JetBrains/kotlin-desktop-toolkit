package org.jetbrains.desktop.linux

@JvmInline
public value class PhysicalPixels(public val rawPhysical: Int)

@JvmInline
public value class LogicalPixels(public val rawLogical: Double) {
    public companion object {
        public val Zero: LogicalPixels = LogicalPixels(0.0)
    }

    public fun toRawPhysical(scale: Scale): Double {
        return scale.rawLogicalToRawPhysical(rawLogical)
    }

    public operator fun compareTo(other: LogicalPixels): Int = rawLogical.compareTo(other.rawLogical)

    public operator fun plus(other: LogicalPixels): LogicalPixels = LogicalPixels(rawLogical + other.rawLogical)
    public operator fun minus(other: LogicalPixels): LogicalPixels = LogicalPixels(rawLogical - other.rawLogical)

    public operator fun div(other: Int): LogicalPixels = LogicalPixels(rawLogical / other)
    public operator fun times(other: Int): LogicalPixels = LogicalPixels(rawLogical * other)
}

@JvmInline
public value class LogicalPixelsInt(public val rawLogical: Int) {
    public companion object {
        public val Zero: LogicalPixelsInt = LogicalPixelsInt(0)
    }

    public fun toLogicalPixels(): LogicalPixels {
        return LogicalPixels(rawLogical.toDouble())
    }

    public fun toRawPhysical(scale: Scale): Double {
        return scale.rawLogicalToRawPhysical(rawLogical.toDouble())
    }

    public operator fun plus(other: LogicalPixelsInt): LogicalPixelsInt = LogicalPixelsInt(rawLogical + other.rawLogical)
    public operator fun minus(other: LogicalPixelsInt): LogicalPixelsInt = LogicalPixelsInt(rawLogical - other.rawLogical)

    public operator fun div(other: Int): LogicalPixelsInt = LogicalPixelsInt(rawLogical / other)
    public operator fun times(other: Int): LogicalPixelsInt = LogicalPixelsInt(rawLogical * other)
}

@JvmInline
public value class Scale private constructor(public val rawScale: Double) {
    public companion object {
        public val NO_SCALE: Scale = Scale(1.0)

        internal fun fromValue120(rawValue120: Int): Scale {
            return Scale(rawValue120.toDouble() / 120.0)
        }
    }

    public fun rawPhysicalToLogical(rawPhysical: Double): LogicalPixels {
        val rawLogical = rawPhysical / rawScale
        return LogicalPixels(rawLogical)
    }

    internal fun rawLogicalToRawPhysical(rawLogical: Double): Double {
        return rawLogical * rawScale
    }
}

public class PhysicalSize internal constructor(
    public val width: PhysicalPixels,
    public val height: PhysicalPixels,
) {
    internal companion object;

    override fun toString(): String {
        return "PhysicalSize(width=${width.rawPhysical}, height=${height.rawPhysical})"
    }

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false

        other as PhysicalSize

        if (width != other.width) return false
        if (height != other.height) return false

        return true
    }

    override fun hashCode(): Int {
        var result = width.hashCode()
        result = 31 * result + height.hashCode()
        return result
    }
}

public data class LogicalSize(
    val width: LogicalPixelsInt,
    val height: LogicalPixelsInt,
) {
    init {
        require(width.rawLogical >= 0 && height.rawLogical >= 0) {
            "Invalid size (both width and height must be positive)"
        }
    }

    public companion object {
        public fun makeWH(width: Int, height: Int): LogicalSize =
            LogicalSize(width = LogicalPixelsInt(width), height = LogicalPixelsInt(height))
    }
}

public class LogicalPoint internal constructor(
    public val x: LogicalPixels,
    public val y: LogicalPixels,
) {
    internal companion object;

    override fun toString(): String {
        return "LogicalPoint(x=${x.rawLogical}, y=${y.rawLogical})"
    }
}

public data class LogicalRect(
    val x: LogicalPixelsInt,
    val y: LogicalPixelsInt,
    val width: LogicalPixelsInt,
    val height: LogicalPixelsInt,
) {
    public companion object {
        public fun makeXYWH(x: Int, y: Int, width: Int, height: Int): LogicalRect = LogicalRect(
            x = LogicalPixelsInt(x),
            y = LogicalPixelsInt(y),
            width = LogicalPixelsInt(width),
            height = LogicalPixelsInt(height),
        )

        public fun makeWH(w: Int, h: Int): LogicalRect =
            LogicalRect(x = LogicalPixelsInt.Zero, y = LogicalPixelsInt.Zero, width = LogicalPixelsInt(w), height = LogicalPixelsInt(h))
    }
}
