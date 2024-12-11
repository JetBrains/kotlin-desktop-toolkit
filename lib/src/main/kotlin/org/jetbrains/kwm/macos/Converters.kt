package org.jetbrains.kwm.macos

import org.jetbrains.kwm.Point
import org.jetbrains.kwm.Size
import java.lang.foreign.MemorySegment
import org.jetbrains.kwm.macos.generated.Size as NativeSize
import org.jetbrains.kwm.macos.generated.Point as NativePoint

internal fun Size.Companion.fromNative(s: MemorySegment): Size {
    return Size(width = NativeSize.width(s),
                height = NativeSize.height(s))
}

internal fun Point.Companion.fromNative(s: MemorySegment): Point {
    return Point(x = NativePoint.x(s),
                 y = NativePoint.y(s))
}