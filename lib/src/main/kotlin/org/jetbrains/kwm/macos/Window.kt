package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

class Window internal constructor(var ptr: MemorySegment): AutoCloseable {
    companion object {
        fun create(title: String, x: Float, y: Float): Window {
            return Arena.ofConfined().use { arena ->
                val title = arena.allocateUtf8String(title)
                val ptr = kwm_macos_h.window_create(title, x, y)
                Window(ptr)
            }
        }
    }

    override fun close() {
        kwm_macos_h.window_deref(ptr)
        ptr = MemorySegment.NULL
    }
}