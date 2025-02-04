package org.jetbrains.desktop.macos

import org.jetbrains.desktop.LogicalPoint
import org.jetbrains.desktop.LogicalSize
import org.jetbrains.desktop.PhysicalSize
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import org.jetbrains.desktop.macos.generated.LogicalSize as NativeLogicalSize
import org.jetbrains.desktop.macos.generated.LogicalPoint as NativeLogicalPoint
import org.jetbrains.desktop.macos.generated.PhysicalSize as NativePhysicalSize
import org.jetbrains.desktop.macos.generated.Color as NativeColor
//import org.jetbrains.kwm.macos.generated.PhysicalPoint as NativePhysicalPoint

internal fun LogicalSize.Companion.fromNative(s: MemorySegment): LogicalSize {
    return LogicalSize(width = NativeLogicalSize.width(s),
                       height = NativeLogicalSize.height(s))
}

internal fun LogicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalSize.allocate(arena)
    NativeLogicalSize.width(result, width)
    NativeLogicalSize.height(result, height)
    return result
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment): LogicalPoint {
    return LogicalPoint(x = NativeLogicalPoint.x(s),
                        y = NativeLogicalPoint.y(s))
}

internal fun LogicalPoint.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalPoint.allocate(arena)
    NativeLogicalPoint.x(result, x)
    NativeLogicalPoint.y(result, y)
    return result
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment): PhysicalSize {
    return PhysicalSize(width = NativePhysicalSize.width(s),
                        height = NativePhysicalSize.height(s))
}

internal fun PhysicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativePhysicalSize.allocate(arena)
    NativePhysicalSize.width(result, width)
    NativePhysicalSize.height(result, height)
    return result
}

//internal fun PhysicalPoint.Companion.fromNative(s: MemorySegment): PhysicalPoint {
//    return PhysicalPoint(x = NativePhysicalPoint.x(s),
//                 y = NativePhysicalPoint.y(s))
//}


internal fun Color.toNative(arena: Arena): MemorySegment {
    val result = NativeColor.allocate(arena)
    NativeColor.red(result, red)
    NativeColor.green(result, green)
    NativeColor.blue(result, blue)
    NativeColor.alpha(result, alpha)
    return result
}