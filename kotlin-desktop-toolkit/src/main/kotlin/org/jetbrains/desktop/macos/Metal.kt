package org.jetbrains.desktop.macos

import org.jetbrains.desktop.PhysicalSize
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class MetalDevice internal constructor(ptr: MemorySegment) : Managed(ptr, desktop_macos_h::metal_deref_device) {
    public companion object {
        public fun create(): MetalDevice {
            return MetalDevice(ffiDownCall { desktop_macos_h.metal_create_device() })
        }
    }

    public val pointerAddress: Long get() = pointer.address()
}

public class MetalCommandQueue internal constructor(ptr: MemorySegment) : Managed(ptr, desktop_macos_h::metal_deref_command_queue) {
    public companion object {
        public fun create(device: MetalDevice): MetalCommandQueue {
            return MetalCommandQueue(ffiDownCall { desktop_macos_h.metal_create_command_queue(device.pointer) })
        }
    }

    public val pointerAddress: Long get() = pointer.address()
}

public class MetalView internal constructor(ptr: MemorySegment) : Managed(ptr, desktop_macos_h::metal_drop_view) {
    public companion object {
        public fun create(device: MetalDevice): MetalView {
            return MetalView(ffiDownCall { desktop_macos_h.metal_create_view(device.pointer) })
        }
    }

    public fun nextTexture(): MetalTexture {
        return MetalTexture(ffiDownCall { desktop_macos_h.metal_view_next_texture(pointer) })
    }

    public fun present(queue: MetalCommandQueue, waitForCATransaction: Boolean) {
        ffiDownCall {
            desktop_macos_h.metal_view_present(pointer, queue.pointer, waitForCATransaction)
        }
    }

    public fun size(): PhysicalSize {
        return Arena.ofConfined().use { arena ->
            PhysicalSize.fromNative(ffiDownCall { desktop_macos_h.metal_view_get_texture_size(arena, pointer) })
        }
    }

    public var isOpaque: Boolean
        get() = ffiDownCall { desktop_macos_h.metal_view_get_is_opaque(pointer) }
        set(value) = ffiDownCall { desktop_macos_h.metal_view_set_is_opaque(pointer, value) }
}

public class MetalTexture internal constructor(ptr: MemorySegment) : Managed(ptr, desktop_macos_h::metal_deref_texture) {
    public val pointerAddress: Long get() = pointer.address()
}
