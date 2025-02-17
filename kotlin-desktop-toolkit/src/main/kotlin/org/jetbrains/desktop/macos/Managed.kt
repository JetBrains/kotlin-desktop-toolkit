package org.jetbrains.desktop.macos

import java.lang.foreign.MemorySegment

public open class Managed internal constructor(
    private var ptr: MemorySegment,
    private val deref: (MemorySegment) -> Unit,
) : AutoCloseable {
    internal val pointer: MemorySegment get() {
        return if (ptr != MemorySegment.NULL) {
            ptr
        } else {
            throw Error("Can't access $this it's aredy closed!")
        }
    }
    override fun close() {
        if (ptr == MemorySegment.NULL) {
            throw Error("Can't close $this it's already closed!")
        }
        ffiDownCall {
            deref(ptr)
        }
        ptr = MemorySegment.NULL
    }
}
