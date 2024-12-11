package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.DisplayLinkCallback
import org.jetbrains.kwm.macos.generated.WindowResizeCallback
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

typealias WindowId = Long;

class Window internal constructor(ptr: MemorySegment): Managed(ptr, kwm_macos_h::window_deref) {
    companion object {
        fun create(title: String, x: Float, y: Float, onResize: () -> Unit = {}): Window {
            val callback = WindowResizeCallback.allocate(onResize, Arena.global()) // todo fixme!!
            return Arena.ofConfined().use { arena ->
                val title = arena.allocateUtf8String(title)
                Window(kwm_macos_h.window_create(title, x, y, callback))
            }
        }
    }

    fun windowId(): WindowId {
        return kwm_macos_h.window_get_window_id(pointer)
    }
}

class DisplayLink internal constructor(ptr: MemorySegment, val arena: Arena): Managed(ptr, kwm_macos_h::display_link_drop) {
    companion object {
        fun createForWindow(window: Window, onNextFrame: () -> Unit): DisplayLink {
            val arena = Arena.ofConfined()
            val callback = DisplayLinkCallback.allocate(onNextFrame, arena)
            return DisplayLink(kwm_macos_h.display_link_create(window.pointer, callback), arena)
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