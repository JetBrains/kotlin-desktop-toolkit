package org.jetbrains.kwm.macos

import java.lang.foreign.MemorySegment

open class Managed(private var ptr: MemorySegment,
                   private val deref: (MemorySegment) -> Unit): AutoCloseable {
    val pointer: MemorySegment get() {
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
        deref(ptr)
        ptr = MemorySegment.NULL
    }
}