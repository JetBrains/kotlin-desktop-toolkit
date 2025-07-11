package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeWindowParams
import org.jetbrains.desktop.win32.generated.desktop_windows_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public typealias WindowId = Long

public data class WindowParams(
    val origin: LogicalPoint = LogicalPoint(0f, 0f),
    val size: LogicalSize = LogicalSize(640f, 480f),
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
            origin: LogicalPoint = LogicalPoint(0f, 0f),
            size: LogicalSize = LogicalSize(640f, 480f),
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

    internal inline fun <T> withPointer(block: (MemorySegment) -> T): T = block(this.ptr)

    public fun windowId(): WindowId {
        return ffiDownCall { desktop_windows_h.window_get_window_id(ptr) }
    }

    public fun getScaleFactor(): Float {
        return ffiDownCall { desktop_windows_h.window_get_scale_factor(ptr) }
    }

    public fun setMinSize(size: LogicalSize) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_windows_h.window_set_min_size(ptr, size.toNative(arena))
            }
        }
    }

    public fun extendContentIntoTitleBar() {
        ffiDownCall { desktop_windows_h.window_extend_content_into_titlebar(ptr) }
    }

    public fun applySystemBackdrop(backdrop: WindowSystemBackdropType) {
        ffiDownCall { desktop_windows_h.window_apply_system_backdrop(ptr, backdrop.toNative()) }
    }

    public fun show() {
        return ffiDownCall { desktop_windows_h.window_show(ptr) }
    }

    public fun setRect(origin: PhysicalPoint, size: PhysicalSize) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_windows_h.window_set_rect(
                    ptr,
                    origin.toNative(arena),
                    size.toNative(arena),
                )
            }
        }
    }

    override fun close() {
        ffiDownCall {
            desktop_windows_h.window_drop(ptr)
        }
    }
}

public enum class WindowSystemBackdropType {
    Auto,
    None,
    Mica,
    DesktopAcrylic,
    MicaAlt,
    ;

    public fun toNative(): Int = when (this) {
        Auto -> desktop_windows_h.NativeWindowSystemBackdropType_Auto()
        None -> desktop_windows_h.NativeWindowSystemBackdropType_None()
        Mica -> desktop_windows_h.NativeWindowSystemBackdropType_Mica()
        DesktopAcrylic -> desktop_windows_h.NativeWindowSystemBackdropType_DesktopAcrylic()
        MicaAlt -> desktop_windows_h.NativeWindowSystemBackdropType_MicaAlt()
    }
}
