package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeEventHandler
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import org.jetbrains.desktop.linux.generated.desktop_linux_h as desktop_h

public typealias WindowId = Int

public class Window internal constructor(
    private val appPtr: MemorySegment,
    private val eventHandler: EventHandler,
    params: WindowParams,
) : AutoCloseable {
    public val windowId: WindowId
    private val arena = Arena.ofConfined()
    private var pointerShape = PointerShape.Default

    private val nativeEventHandler = NativeEventHandler.allocate(::onEvent, arena)

    init {
        Arena.ofConfined().use { arena ->
            windowId = ffiDownCall {
                desktop_h.window_create(appPtr, nativeEventHandler, params.toNative(arena))
            }
        }
    }

    // called from native
    private fun onEvent(nativeEvent: MemorySegment): Boolean {
//        println("onEvent called")
        val event = Event.fromNative(nativeEvent)
        return ffiUpCall(defaultResult = false) {
            val result = eventHandler(event)
            when (result) {
                EventHandlerResult.Continue -> false
                EventHandlerResult.Stop -> true
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

    override fun close() {
        ffiDownCall {
            desktop_h.window_close(appPtr, windowId)
        }
        arena.close()
    }
}
