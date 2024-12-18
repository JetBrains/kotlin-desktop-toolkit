package org.jetbrains.kwm.macos

import org.jetbrains.kwm.LogicalPoint
import org.jetbrains.kwm.LogicalSize
import org.jetbrains.kwm.PhysicalSize
import java.lang.foreign.MemorySegment
import org.jetbrains.kwm.macos.generated.LogicalSize as NativeLogicalSize
import org.jetbrains.kwm.macos.generated.LogicalPoint as NativeLogicalPoint
import org.jetbrains.kwm.macos.generated.PhysicalSize as NativePhysicalSize
//import org.jetbrains.kwm.macos.generated.PhysicalPoint as NativePhysicalPoint

internal fun LogicalSize.Companion.fromNative(s: MemorySegment): LogicalSize {
    return LogicalSize(width = NativeLogicalSize.width(s),
                height = NativeLogicalSize.height(s))
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment): LogicalPoint {
    return LogicalPoint(x = NativeLogicalPoint.x(s),
                 y = NativeLogicalPoint.y(s))
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment): PhysicalSize {
    return PhysicalSize(width = NativePhysicalSize.width(s),
                height = NativePhysicalSize.height(s))
}

//internal fun PhysicalPoint.Companion.fromNative(s: MemorySegment): PhysicalPoint {
//    return PhysicalPoint(x = NativePhysicalPoint.x(s),
//                 y = NativePhysicalPoint.y(s))
//}