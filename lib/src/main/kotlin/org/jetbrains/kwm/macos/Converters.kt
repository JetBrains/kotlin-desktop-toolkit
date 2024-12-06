package org.jetbrains.kwm.macos

import Size
import java.lang.foreign.MemorySegment
import org.jetbrains.kwm.macos.generated.Size as NativeSize

internal fun Size.Companion.fromNative(s: MemorySegment): Size {
    return Size(width = NativeSize.width(s),
                height = NativeSize.height(s))
}