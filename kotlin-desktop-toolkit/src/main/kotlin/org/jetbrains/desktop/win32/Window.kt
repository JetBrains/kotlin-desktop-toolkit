package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeWindowParams
import org.jetbrains.desktop.win32.generated.NativeWindowStyle
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.concurrent.atomics.fetchAndIncrement

public typealias WindowId = Long

public data class WindowParams(
    val origin: LogicalPoint = LogicalPoint(0f, 0f),
    val size: LogicalSize = LogicalSize(640f, 480f),
    val title: String = "",
    val style: WindowStyle = WindowStyle(),
) {
    internal fun toNative(arena: Arena): MemorySegment = NativeWindowParams.allocate(arena).also { nativeWindowParams ->
        NativeWindowParams.origin(nativeWindowParams, origin.toNative(arena))
        NativeWindowParams.size(nativeWindowParams, size.toNative(arena))
        NativeWindowParams.title(nativeWindowParams, arena.allocateUtf8String(title))
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

public class Window internal constructor(
    private val windowId: WindowId,
    private val ptr: MemorySegment,
) : AutoCloseable {
    public companion object {
        @OptIn(ExperimentalAtomicApi::class)
        private val nextWindowId: AtomicLong = AtomicLong(1)

        @OptIn(ExperimentalAtomicApi::class)
        public fun new(appPtr: MemorySegment): Window {
            val windowId = nextWindowId.fetchAndIncrement()
            val ptr = Arena.ofConfined().use { arena ->
                ffiDownCall {
                    desktop_win32_h.window_new(appPtr, windowId)
                }
            }
            return Window(windowId, ptr)
        }
    }

    internal inline fun <T> withPointer(block: (MemorySegment) -> T): T = block(this.ptr)

    public val id: WindowId get() = this.windowId

    public fun create(params: WindowParams): Unit = Arena.ofConfined().use { arena ->
        ffiDownCall {
            desktop_win32_h.window_create(ptr, params.toNative(arena))
        }
    }

    public fun create(
        origin: LogicalPoint = LogicalPoint(0f, 0f),
        size: LogicalSize = LogicalSize(640f, 480f),
        title: String = "Window",
        style: WindowStyle = WindowStyle(),
    ) {
        create(
            WindowParams(
                origin,
                size,
                title,
                style,
            ),
        )
    }

    public fun getScaleFactor(): Float {
        return ffiDownCall { desktop_win32_h.window_get_scale_factor(ptr) }
    }

    public fun getScreen(): Screen {
        return Arena.ofConfined().use { arena ->
            val screenInfo = ffiDownCall { desktop_win32_h.window_get_screen_info(arena, ptr) }
            try {
                Screen.fromNative(screenInfo)
            } finally {
                ffiDownCall { desktop_win32_h.screen_info_drop(screenInfo) }
            }
        }
    }

    public fun setMinSize(size: LogicalSize) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_win32_h.window_set_min_size(ptr, size.toNative(arena))
            }
        }
    }

    public fun show() {
        return ffiDownCall { desktop_win32_h.window_show(ptr) }
    }

    public fun setRect(origin: LogicalPoint, size: LogicalSize) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_win32_h.window_set_rect(
                    ptr,
                    origin.toNative(arena),
                    size.toNative(arena),
                )
            }
        }
    }

    public fun requestRedraw() {
        ffiDownCall { desktop_win32_h.window_request_redraw(ptr) }
    }

    public fun requestClose() {
        ffiDownCall { desktop_win32_h.window_request_close(ptr) }
    }

    override fun close() {
        ffiDownCall {
            desktop_win32_h.window_drop(ptr)
        }
    }
}

public enum class WindowTitleBarKind {
    System,
    Custom,
    None,
    ;

    public fun toNative(): Int = when (this) {
        System -> desktop_win32_h.NativeWindowTitleBarKind_System()
        Custom -> desktop_win32_h.NativeWindowTitleBarKind_Custom()
        None -> desktop_win32_h.NativeWindowTitleBarKind_None()
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
        Auto -> desktop_win32_h.NativeWindowSystemBackdropType_Auto()
        None -> desktop_win32_h.NativeWindowSystemBackdropType_None()
        Mica -> desktop_win32_h.NativeWindowSystemBackdropType_Mica()
        DesktopAcrylic -> desktop_win32_h.NativeWindowSystemBackdropType_DesktopAcrylic()
        MicaAlt -> desktop_win32_h.NativeWindowSystemBackdropType_MicaAlt()
    }
}
