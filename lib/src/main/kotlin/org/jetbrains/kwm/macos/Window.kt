package org.jetbrains.kwm.macos

import org.jetbrains.kwm.LogicalSize
import org.jetbrains.kwm.LogicalPoint
import org.jetbrains.kwm.macos.generated.WindowParams as NativeWindowParams
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

typealias WindowId = Long;

class Window internal constructor(ptr: MemorySegment): Managed(ptr, kwm_macos_h::window_drop) {
    data class WindowParams(val origin: LogicalPoint,
                            val size: LogicalSize,
                            val title: String) {
        internal fun toNative(arena: Arena): MemorySegment {
            val nativeWindowParams = NativeWindowParams.allocate(arena)
            NativeWindowParams.origin(nativeWindowParams, origin.toNative(arena))
            NativeWindowParams.size(nativeWindowParams, size.toNative(arena))
            NativeWindowParams.title(nativeWindowParams, arena.allocateUtf8String(title))
            return nativeWindowParams
        }
    }

    companion object {
        fun create(params: WindowParams): Window {
            return Arena.ofConfined().use { arena ->
                Window(kwm_macos_h.window_create(params.toNative(arena)))
            }
        }

        fun create(origin: LogicalPoint = LogicalPoint(0.0, 0.0),
                   size: LogicalSize = LogicalSize(640.0, 480.0),
                   title: String = "Window"): Window {
            return create(WindowParams(origin, size, title))
        }
    }

    fun windowId(): WindowId {
        return kwm_macos_h.window_get_window_id(pointer)
    }

    fun screenId(): ScreenId {
        return kwm_macos_h.window_get_screen_id(pointer)
    }

    fun scaleFactor(): Double {
        return kwm_macos_h.window_scale_factor(pointer)
    }

    fun attachView(layer: MetalView) {
        kwm_macos_h.window_attach_layer(pointer, layer.pointer)
    }
}