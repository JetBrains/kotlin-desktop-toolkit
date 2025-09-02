package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeWindowParams
import org.jetbrains.desktop.win32.generated.NativeWindowStyle
import org.jetbrains.desktop.win32.generated.desktop_windows_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public typealias WindowId = Long

public data class WindowParams(
    val origin: LogicalPoint = LogicalPoint(0f, 0f),
    val size: LogicalSize = LogicalSize(640f, 480f),
    val title: String? = null,
    val style: WindowStyle = WindowStyle(),
) {
    internal fun toNative(arena: Arena): MemorySegment = NativeWindowParams.allocate(arena).also { nativeWindowParams ->
        NativeWindowParams.origin(nativeWindowParams, origin.toNative(arena))
        NativeWindowParams.size(nativeWindowParams, size.toNative(arena))
        NativeWindowParams.title(nativeWindowParams, title?.let(arena::allocateUtf8String) ?: MemorySegment.NULL)
        NativeWindowParams.style(nativeWindowParams, style.toNative(arena))
    }
}

public data class WindowStyle(
    public val titleBarKind: WindowTitleBarKind = WindowTitleBarKind.System,

    public val isResizable: Boolean = true,
    public val isMinimizable: Boolean = true,
    public val isMaximizable: Boolean = true,

    public val systemBackdropType: WindowSystemBackdropType = WindowSystemBackdropType.Auto,
) {
    internal fun toNative(arena: Arena): MemorySegment = NativeWindowStyle.allocate(arena).also { nativeWindowStyle ->
        NativeWindowStyle.title_bar_kind(nativeWindowStyle, titleBarKind.toNative())

        NativeWindowStyle.is_resizable(nativeWindowStyle, isResizable)
        NativeWindowStyle.is_minimizable(nativeWindowStyle, isMinimizable)
        NativeWindowStyle.is_maximizable(nativeWindowStyle, isMaximizable)

        NativeWindowStyle.system_backdrop_type(nativeWindowStyle, systemBackdropType.toNative())
    }
}

public class Window internal constructor(private val ptr: MemorySegment) : AutoCloseable {
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
            style: WindowStyle = WindowStyle(),
        ): Window {
            return create(
                appPtr,
                WindowParams(
                    origin,
                    size,
                    title,
                    style,
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

    public fun requestUpdate() {
        ffiDownCall { desktop_windows_h.window_request_update(ptr) }
    }

    override fun close() {
        ffiDownCall {
            desktop_windows_h.window_drop(ptr)
        }
    }
}

public enum class WindowTitleBarKind {
    System,
    Custom,
    None,
    ;

    public fun toNative(): Int = when (this) {
        System -> desktop_windows_h.NativeWindowTitleBarKind_System()
        Custom -> desktop_windows_h.NativeWindowTitleBarKind_Custom()
        None -> desktop_windows_h.NativeWindowTitleBarKind_None()
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
