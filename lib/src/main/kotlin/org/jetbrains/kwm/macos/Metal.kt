package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.kwm_macos_h
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
}

class MetalView internal constructor(ptr: MemorySegment): Managed(ptr, kwm_macos_h::metal_deref_view) {
    companion object {
        fun create(device: MetalDevice): MetalView {
            return MetalView(kwm_macos_h.metal_create_view(device.pointer))
        }
    }

    fun attachToWindow(window: Window) {
        kwm_macos_h.metal_view_attach_to_window(pointer, window.pointer)
    }
}