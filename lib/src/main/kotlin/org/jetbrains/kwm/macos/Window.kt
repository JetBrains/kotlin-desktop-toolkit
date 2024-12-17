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

    fun screenId(): ScreenId {
        return kwm_macos_h.window_get_screen_id(pointer)
    }

    fun attachView(layer: MetalView) {
        kwm_macos_h.window_attach_layer(pointer, layer.pointer)
    }
}