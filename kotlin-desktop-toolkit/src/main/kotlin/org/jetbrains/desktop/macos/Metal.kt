package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h
import org.jetbrains.desktop.PhysicalSize
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

class MetalDevice internal constructor(ptr: MemorySegment): Managed(ptr, desktop_macos_h::metal_deref_device) {
    companion object {
        fun create(): MetalDevice {
            return MetalDevice(ffiDownCall { desktop_macos_h.metal_create_device() })
        }
    }

    val pointerAddress get() = pointer.address()
}

class MetalCommandQueue internal constructor(ptr: MemorySegment): Managed(ptr, desktop_macos_h::metal_deref_command_queue) {
    companion object {
        fun create(device: MetalDevice): MetalCommandQueue {
            return MetalCommandQueue(ffiDownCall { desktop_macos_h.metal_create_command_queue(device.pointer) })
        }
    }

    val pointerAddress get() = pointer.address()
}

class MetalView internal constructor(ptr: MemorySegment): Managed(ptr, desktop_macos_h::metal_drop_view) {
    companion object {
        fun create(device: MetalDevice): MetalView {
            return MetalView(ffiDownCall { desktop_macos_h.metal_create_view(device.pointer) })
        }
    }

    fun nextTexture(): MetalTexture {
        return MetalTexture(ffiDownCall { desktop_macos_h.metal_view_next_texture(pointer) })
    }

    fun present(queue: MetalCommandQueue, waitForCATransaction: Boolean) {
        ffiDownCall {
            desktop_macos_h.metal_view_present(pointer, queue.pointer, waitForCATransaction)
        }
    }

    fun size(): PhysicalSize {
        return Arena.ofConfined().use { arena ->
            PhysicalSize.fromNative(ffiDownCall { desktop_macos_h.metal_view_get_texture_size(arena, pointer) })
        }
    }

    var isOpaque: Boolean
        get() = ffiDownCall { desktop_macos_h.metal_view_get_is_opaque(pointer) }
        set(value) = ffiDownCall { desktop_macos_h.metal_view_set_is_opaque(pointer, value) }
}

class MetalTexture internal constructor(ptr: MemorySegment): Managed(ptr, desktop_macos_h::metal_deref_texture) {
    val pointerAddress get() = pointer.address()
}