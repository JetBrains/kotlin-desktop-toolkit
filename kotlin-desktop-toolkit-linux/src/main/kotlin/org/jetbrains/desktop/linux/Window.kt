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
    private var scale: Double = 1.0

    private val nativeEventHandler = NativeEventHandler.allocate(::onEvent, arena)

    init {
        windowId = ffiDownCall {
            desktop_h.window_create(appPtr, nativeEventHandler, params.toNative(arena))
        }
    }

    // called from native
    private fun onEvent(nativeEvent: MemorySegment): Boolean {
//        println("onEvent called")
        val event = Event.fromNative(nativeEvent)
        when (event) {
            is Event.WindowScaleChanged -> {
                scale = event.newScale
            }
            else -> {}
        }
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

//    public fun screenId(): ScreenId {
//        return ffiDownCall {
//            desktop_h.window_get_screen_id(pointer)
//        }
//    }

    public fun scaleFactor(): Double = scale

//    public var title: String
//        get() {
//            val title = ffiDownCall { desktop_h.window_get_title(pointer) }
//            return try {
//                title.getUtf8String(0)
//            } finally {
//                ffiDownCall { desktop_h.string_drop(title) }
//            }
//        }
//        set(value) {
//            Arena.ofConfined().use { arena ->
//                val title = arena.allocateUtf8String(value)
//                ffiDownCall { desktop_h.window_set_title(pointer, title) }
//            }
//        }

//    public val origin: LogicalPoint
//        get() {
//            return Arena.ofConfined().use { arena ->
//                LogicalPoint.fromNative(ffiDownCall { desktop_h.window_get_origin(arena, appPtr, windowId) })
//            }
//        }

    public val size: LogicalSize
        get() {
            return Arena.ofConfined().use { arena ->
                LogicalSize.fromNative(ffiDownCall { desktop_h.window_get_size(arena, appPtr, windowId) })
            }
        }

//    public val contentOrigin: LogicalPoint
//        get() {
//            return Arena.ofConfined().use { arena ->
//                LogicalPoint.fromNative(ffiDownCall { desktop_h.window_get_content_origin(arena, pointer) })
//            }
//        }
//
//    public val contentSize: LogicalSize
//        get() {
//            return Arena.ofConfined().use { arena ->
//                LogicalSize.fromNative(ffiDownCall { desktop_h.window_get_content_size(arena, pointer) })
//            }
//        }
//
//    public var maxSize: LogicalSize
//        get() {
//            return Arena.ofConfined().use { arena ->
//                LogicalSize.fromNative(ffiDownCall { desktop_h.window_get_max_size(arena, pointer) })
//            }
//        }
//        set(value) {
//            Arena.ofConfined().use { arena ->
//                ffiDownCall {
//                    desktop_h.window_set_max_size(pointer, value.toNative(arena))
//                }
//            }
//        }
//
//    public var minSize: LogicalSize
//        get() {
//            return Arena.ofConfined().use { arena ->
//                LogicalSize.fromNative(ffiDownCall { desktop_h.window_get_min_size(arena, pointer) })
//            }
//        }
//        set(value) {
//            Arena.ofConfined().use { arena ->
//                ffiDownCall { desktop_h.window_set_min_size(pointer, value.toNative(arena)) }
//            }
//        }
//
//    public val isFullScreen: Boolean
//        get() {
//            return ffiDownCall { desktop_h.window_is_full_screen(pointer) }
//        }
//
//    public fun toggleFullScreen() {
//        ffiDownCall {
//            desktop_h.window_toggle_full_screen(pointer)
//        }
//    }
//
//    public val isKey: Boolean
//        get() {
//            return ffiDownCall { desktop_h.window_is_key(appPtr, windowId) }
//        }
//
//    public val isMain: Boolean
//        get() {
//            return ffiDownCall { desktop_h.window_is_main(appPtr, windowId) }
//        }

    override fun close() {
        ffiDownCall {
            desktop_h.window_drop(appPtr, windowId)
        }
        arena.close()
    }
}
