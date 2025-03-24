package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeKeyModifiers
import org.jetbrains.desktop.linux.generated.NativeLogicalPoint
import org.jetbrains.desktop.linux.generated.NativeLogicalSize
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

internal fun LogicalSize.Companion.fromNative(s: MemorySegment) = LogicalSize(
    width = NativeLogicalSize.width(s),
    height = NativeLogicalSize.height(s),
)

internal fun LogicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalSize.allocate(arena)
    NativeLogicalSize.width(result, width)
    NativeLogicalSize.height(result, height)
    return result
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment) = LogicalPoint(
    x = NativeLogicalPoint.x(s),
    y = NativeLogicalPoint.y(s),
)

internal fun LogicalPoint.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalPoint.allocate(arena)
    NativeLogicalPoint.x(result, x)
    NativeLogicalPoint.y(result, y)
    return result
}

internal fun KeyModifiers.Companion.fromNative(s: MemorySegment) = KeyModifiers(
    capsLock = NativeKeyModifiers.caps_lock(s),
    shift = NativeKeyModifiers.shift(s),
    control = NativeKeyModifiers.ctrl(s),
    alt = NativeKeyModifiers.alt(s),
    logo = NativeKeyModifiers.logo(s),
    numLock = NativeKeyModifiers.num_lock(s),
)
