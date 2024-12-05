package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.MetalViewDrawCallback
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

class MetalDevice internal constructor(ptr: MemorySegment): Managed(ptr, kwm_macos_h::metal_deref_device) {
    companion object {
        fun create(): MetalDevice {
            return MetalDevice(kwm_macos_h.metal_create_device())
        }
    }
}

class MetalCommandQueue internal constructor(ptr: MemorySegment): Managed(ptr, kwm_macos_h::metal_deref_command_queue) {
    companion object {
        fun create(device: MetalDevice): MetalCommandQueue {
            return MetalCommandQueue(kwm_macos_h.metal_create_command_queue(device.pointer))
        }
    }

    fun present(view: MetalView) {
        kwm_macos_h.metal_command_queue_present(pointer, view.pointer)
    }
}

class MetalView internal constructor(ptr: MemorySegment, val arena: Arena): Managed(ptr, kwm_macos_h::metal_deref_view) {
    companion object {
        fun create(device: MetalDevice, onDraw: () -> Unit): MetalView {
            val arena = Arena.ofConfined()
            val onDraw = MetalViewDrawCallback.allocate(onDraw, arena)
            return MetalView(kwm_macos_h.metal_create_view(device.pointer, onDraw), arena)
        }
    }

    fun attachToWindow(window: Window) {
        kwm_macos_h.metal_view_attach_to_window(pointer, window.pointer)
    }

    override fun close() {
        super.close()
        arena.close()
    }
}