package org.jetbrains.kwm.macos

import org.jetbrains.kwm.LogicalSize
import org.jetbrains.kwm.PhysicalSize
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

    fun commit() {
        kwm_macos_h.metal_command_queue_commit(pointer)
    }
}

class MetalView internal constructor(ptr: MemorySegment): Managed(ptr, kwm_macos_h::metal_drop_view) {
    companion object {
        fun create(device: MetalDevice): MetalView {
            return MetalView(kwm_macos_h.metal_create_view(device.pointer))
        }
    }

    fun nextTexture(): MetalTexture {
        return MetalTexture(kwm_macos_h.metal_view_next_texture(pointer))
    }

    fun present() {
        kwm_macos_h.metal_view_present(pointer)
    }

    fun size(): PhysicalSize {
        return Arena.ofConfined().use { arena ->
            PhysicalSize.fromNative(kwm_macos_h.metal_view_get_texture_size(arena, pointer))
        }
    }
}

class MetalTexture internal constructor(ptr: MemorySegment): Managed(ptr, kwm_macos_h::metal_deref_texture)