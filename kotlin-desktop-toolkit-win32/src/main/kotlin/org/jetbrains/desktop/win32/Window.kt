package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeWindowParams
import org.jetbrains.desktop.win32.generated.desktop_windows_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public typealias WindowId = Long

public data class WindowParams(
    val origin: PhysicalPoint = PhysicalPoint(0, 0),
    val size: PhysicalSize = PhysicalSize(640, 480),
    val title: String = "Window",
    val isResizable: Boolean = true,
    val isClosable: Boolean = true,
    val isMinimizable: Boolean = true,
) {
    internal fun toNative(arena: Arena): MemorySegment {
        val nativeWindowParams = NativeWindowParams.allocate(arena)
        NativeWindowParams.origin(nativeWindowParams, origin.toNative(arena))
        NativeWindowParams.size(nativeWindowParams, size.toNative(arena))
        NativeWindowParams.title(nativeWindowParams, arena.allocateUtf8String(title))

        NativeWindowParams.is_resizable(nativeWindowParams, isResizable)
        NativeWindowParams.is_closable(nativeWindowParams, isClosable)
        NativeWindowParams.is_minimizable(nativeWindowParams, isMinimizable)
        return nativeWindowParams
    }
}

public class Window internal constructor(
    private val ptr: MemorySegment,
) : AutoCloseable {
    public companion object {
        public fun create(appPtr: MemorySegment, params: WindowParams): Window {
            return Arena.ofConfined().use { arena ->
                Window(
                    ffiDownCall {
                        desktop_windows_h.window_create(appPtr, params.toNative(arena))
                    },
                )
            }
        }

        public fun create(
            appPtr: MemorySegment,
            origin: PhysicalPoint = PhysicalPoint(0, 0),
            size: PhysicalSize = PhysicalSize(640, 480),
            title: String = "Window",
            isResizable: Boolean = true,
            isClosable: Boolean = true,
            isMinimizable: Boolean = true,
        ): Window {
            return create(
                appPtr,
                WindowParams(
                    origin,
                    size,
                    title,
                    isResizable,
                    isClosable,
                    isMinimizable,
                ),
            )
        }
    }

    public fun show() {
        return ffiDownCall { desktop_windows_h.window_show(ptr) }
    }

    override fun close() {
        ffiDownCall {
            desktop_windows_h.window_drop(ptr)
        }
    }
}
