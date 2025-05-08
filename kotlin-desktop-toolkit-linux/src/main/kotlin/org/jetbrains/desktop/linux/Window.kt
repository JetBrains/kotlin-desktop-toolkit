package org.jetbrains.desktop.linux

import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import org.jetbrains.desktop.linux.generated.desktop_linux_h as desktop_h

public typealias WindowId = Long

public class Window internal constructor(
    private val appPtr: MemorySegment,
    params: WindowParams,
) : AutoCloseable {
    public val windowId: WindowId = params.windowId
    private var pointerShape = PointerShape.Default

    init {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_h.window_create(appPtr, params.toNative(arena))
            }
        }
    }

    public fun windowId(): WindowId {
        return windowId
    }

    public fun getSize(): LogicalSize {
        return Arena.ofConfined().use { arena ->
            LogicalSize.fromNative(ffiDownCall { desktop_h.window_get_size(arena, appPtr, windowId) })
        }
    }

    public fun setTitle(title: String) {
        Arena.ofConfined().use { arena ->
            val nativeTitle = arena.allocateUtf8String(title)
            ffiDownCall { desktop_h.window_set_title(appPtr, windowId, nativeTitle) }
        }
    }

    public fun setFullScreen() {
        ffiDownCall {
            desktop_h.window_set_fullscreen(appPtr, windowId)
        }
    }

    public fun unsetFullScreen() {
        ffiDownCall {
            desktop_h.window_unset_fullscreen(appPtr, windowId)
        }
    }

    public fun maximize() {
        ffiDownCall {
            desktop_h.window_maximize(appPtr, windowId)
        }
    }

    public fun unmaximize() {
        ffiDownCall {
            desktop_h.window_unmaximize(appPtr, windowId)
        }
    }

    public fun minimize() {
        ffiDownCall {
            desktop_h.window_minimize(appPtr, windowId)
        }
    }

    public fun startMove() {
        ffiDownCall {
            desktop_h.window_start_move(appPtr, windowId)
        }
    }

    public fun startResize(edge: WindowResizeEdge) {
        ffiDownCall {
            desktop_h.window_start_resize(appPtr, windowId, edge.toNative())
        }
    }

    public fun showMenu(position: LogicalPoint) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_h.window_show_menu(appPtr, windowId, position.toNative(arena))
            }
        }
    }

//    public val isKey: Boolean
//        get() {
//            return ffiDownCall { desktop_h.window_is_key(appPtr, windowId) }
//        }
//
//    public val isMain: Boolean
//        get() {
//            return ffiDownCall { desktop_h.window_is_main(appPtr, windowId) }
//        }

    public fun setMinSize(size: LogicalSize) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_h.window_set_min_size(appPtr, windowId, size.toNative(arena))
            }
        }
    }

    public fun setMaxSize(size: LogicalSize) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_h.window_set_max_size(appPtr, windowId, size.toNative(arena))
            }
        }
    }

    public fun setPointerShape(shape: PointerShape) {
        if (pointerShape != shape) {
            pointerShape = shape
            ffiDownCall {
                desktop_h.window_set_pointer_shape(appPtr, windowId, shape.toNative())
            }
        }
    }

    public fun startDrag(data: ClipboardData) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_h.window_start_drag(appPtr, windowId, data.toNative(arena))
            }
        }
    }

    override fun close() {
        Logger.trace { "Window: closing window with id $windowId" }
        ffiDownCall {
            desktop_h.window_close(appPtr, windowId)
        }
        Logger.trace { "Window: closed window with id $windowId" }
    }
}
