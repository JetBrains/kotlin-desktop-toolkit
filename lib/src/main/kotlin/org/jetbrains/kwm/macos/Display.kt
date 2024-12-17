package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.DisplayLinkCallback
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

typealias ScreenId = Int

class DisplayLink internal constructor(ptr: MemorySegment, val arena: Arena): Managed(ptr, kwm_macos_h::display_link_drop) {
    companion object {
        fun create(screenId: ScreenId, onNextFrame: () -> Unit): DisplayLink {
            val arena = Arena.ofConfined()
            val callback = DisplayLinkCallback.allocate(onNextFrame, arena)
            return DisplayLink(kwm_macos_h.display_link_create(screenId, callback), arena)
        }
    }

    fun setPaused(value: Boolean) {
        kwm_macos_h.display_link_set_paused(pointer, value);
    }

    override fun close() {
        super.close()
        arena.close()
    }
}
