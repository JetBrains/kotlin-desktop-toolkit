package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeLogicalPoint
import org.jetbrains.desktop.win32.generated.NativeLogicalSize
import org.jetbrains.desktop.win32.generated.NativePhysicalPoint
import org.jetbrains.desktop.win32.generated.NativePhysicalSize
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

internal fun LogicalSize.toNative(arena: Arena): MemorySegment = NativeLogicalSize.allocate(arena).also { result ->
    NativeLogicalSize.width(result, width)
    NativeLogicalSize.height(result, height)
}

internal fun LogicalPoint.toNative(arena: Arena): MemorySegment = NativeLogicalPoint.allocate(arena).also { result ->
    NativeLogicalPoint.x(result, x)
    NativeLogicalPoint.y(result, y)
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment) = PhysicalSize(
    width = NativePhysicalSize.width(s),
    height = NativePhysicalSize.height(s),
)

internal fun PhysicalPoint.Companion.fromNative(s: MemorySegment) = PhysicalPoint(
    x = NativePhysicalPoint.x(s),
    y = NativePhysicalPoint.y(s),
)

internal fun PhysicalSize.toNative(arena: Arena): MemorySegment = NativePhysicalSize.allocate(arena).also { result ->
    NativePhysicalSize.width(result, width)
    NativePhysicalSize.height(result, height)
}

internal fun PhysicalPoint.toNative(arena: Arena): MemorySegment = NativePhysicalPoint.allocate(arena).also { result ->
    NativePhysicalPoint.x(result, x)
    NativePhysicalPoint.y(result, y)
}
