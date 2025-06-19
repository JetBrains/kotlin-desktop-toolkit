package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativePhysicalPoint
import org.jetbrains.desktop.win32.generated.NativePhysicalSize
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

internal fun PhysicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativePhysicalSize.allocate(arena)
    NativePhysicalSize.width(result, width)
    NativePhysicalSize.height(result, height)
    return result
}

internal fun PhysicalPoint.toNative(arena: Arena): MemorySegment {
    val result = NativePhysicalPoint.allocate(arena)
    NativePhysicalPoint.x(result, x)
    NativePhysicalPoint.y(result, y)
    return result
}
