package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.desktop_linux_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public typealias WindowId = Long

public enum class DragAction {
    Copy,
    Move,
    Ask,
    ;

    internal companion object
}

public class Window internal constructor(
    private val appPtr: MemorySegment,
    params: WindowParams,
) : AutoCloseable {
    public val windowId: WindowId = params.windowId
    private var pointerShape = PointerShape.Default

    init {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.window_create(appPtr, params.toNative(arena))
            }
        }
    }

    override fun toString(): String {
        return "${javaClass.typeName}(windowId=$windowId, appPtr=0x${appPtr.address().toString(16)})"
    }

    public fun windowId(): WindowId {
        return windowId
    }

    public fun getSize(): LogicalSize {
        return Arena.ofConfined().use { arena ->
            LogicalSize.fromNative(ffiDownCall { desktop_linux_h.window_get_size(arena, appPtr, windowId) })
        }
    }

    public fun setTitle(title: String) {
        Arena.ofConfined().use { arena ->
            val nativeTitle = arena.allocateUtf8String(title)
            ffiDownCall { desktop_linux_h.window_set_title(appPtr, windowId, nativeTitle) }
        }
    }

    public fun setFullScreen() {
        ffiDownCall {
            desktop_linux_h.window_set_fullscreen(appPtr, windowId)
        }
    }

    public fun unsetFullScreen() {
        ffiDownCall {
            desktop_linux_h.window_unset_fullscreen(appPtr, windowId)
        }
    }

    public fun maximize() {
        ffiDownCall {
            desktop_linux_h.window_maximize(appPtr, windowId)
        }
    }

    public fun unmaximize() {
        ffiDownCall {
            desktop_linux_h.window_unmaximize(appPtr, windowId)
        }
    }

    public fun minimize() {
        ffiDownCall {
            desktop_linux_h.window_minimize(appPtr, windowId)
        }
    }

    public fun startMove() {
        ffiDownCall {
            desktop_linux_h.window_start_move(appPtr, windowId)
        }
    }

    public fun startResize(edge: WindowResizeEdge) {
        ffiDownCall {
            desktop_linux_h.window_start_resize(appPtr, windowId, edge.toNative())
        }
    }

    public fun showMenu(position: LogicalPoint) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.window_show_menu(appPtr, windowId, position.toNative(arena))
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
                desktop_linux_h.window_set_min_size(appPtr, windowId, size.toNative(arena))
            }
        }
    }

    public fun setMaxSize(size: LogicalSize) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.window_set_max_size(appPtr, windowId, size.toNative(arena))
            }
        }
    }

    public fun setPointerShape(shape: PointerShape) {
        if (pointerShape != shape) {
            pointerShape = shape
            ffiDownCall {
                desktop_linux_h.window_set_pointer_shape(appPtr, windowId, shape.toNative())
            }
        }
    }

    /**
     * Start a drag&drop action with the data that can be interpreted in any of the provided MIME type formats.
     * Later, [ApplicationConfig.getDataTransferData] may be called, with [DataSource.DragAndDrop] argument,
     * to actually get the data with the specified MIME type.
     */
    public fun startDrag(mimeTypes: List<String>, action: DragAction) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.application_start_drag_and_drop(appPtr, windowId, mimeTypesToNative(arena, mimeTypes), action.toNative())
            }
        }
    }

    /** Will produce [Event.DataTransfer] event if there is clipboard content. */
    public fun clipboardPaste(serial: Int, supportedMimeTypes: List<String>): Boolean {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.window_clipboard_paste(appPtr, windowId, serial, mimeTypesToNative(arena, supportedMimeTypes))
            }
        }
    }

    override fun close() {
        Logger.trace { "Window: closing window with id $windowId" }
        ffiDownCall {
            desktop_linux_h.window_close(appPtr, windowId)
        }
        Logger.trace { "Window: closed window with id $windowId" }
    }
}
