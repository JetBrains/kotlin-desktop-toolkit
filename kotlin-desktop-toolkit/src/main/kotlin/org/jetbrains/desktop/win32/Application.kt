package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.win32.generated.NativeEventHandler
import org.jetbrains.desktop.win32.generated.`application_dispatcher_invoke$callback`
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.ConcurrentLinkedQueue

public class Application : AutoCloseable {
    private val arena: Arena

    private val callbacksQueue: ConcurrentLinkedQueue<() -> Unit>
    private val callback: MemorySegment

    private var ptr: MemorySegment? = null
    private var eventHandler: EventHandler? = null

    init {
        arena = Arena.ofShared()
        callbacksQueue = ConcurrentLinkedQueue()
        callback = `application_dispatcher_invoke$callback`.allocate(::pollCallbacks, arena)
    }

    private val appPtr: MemorySegment get() = ptr ?: error("App has not been initialized yet")

    public fun invokeOnDispatcher(body: () -> Unit): Unit = when (ptr) {
        null -> error("App has not been initialized yet; use the [onStartup] method instead.")
        else -> {
            assert(callbacksQueue.offer(body))
            ffiDownCall {
                desktop_win32_h.application_dispatcher_invoke(ptr, callback)
            }
        }
    }

    public fun onStartup(handler: () -> Unit): Unit = when (ptr) {
        null -> assert(callbacksQueue.offer(handler))
        else -> error("App has already been initialized; use the [invokeOnDispatcher] method instead.")
    }

    public fun runEventLoop(eventHandler: EventHandler) {
        this.eventHandler = eventHandler
        ptr = ffiDownCall {
            desktop_win32_h.application_init(applicationCallbacks())
        }
        if (callbacksQueue.isNotEmpty()) {
            ffiDownCall {
                desktop_win32_h.application_dispatcher_invoke(appPtr, callback)
            }
        }
        ffiDownCall {
            desktop_win32_h.application_run_event_loop(appPtr)
        }
    }

    public fun stopEventLoop() {
        ffiDownCall {
            desktop_win32_h.application_stop_event_loop(appPtr)
        }
    }

    public fun createWindow(params: WindowParams): Window {
        return Window.create(appPtr, params)
    }

    private fun runEventHandler(windowId: WindowId, event: Event): EventHandlerResult {
        return eventHandler?.let { eventHandler ->
            eventHandler(windowId, event)
        } ?: run {
            Logger.warn { "eventHandler is null; event: $event was ignored!" }
            EventHandlerResult.Continue
        }
    }

    // called from native
    private fun onEvent(windowId: WindowId, nativeEvent: MemorySegment): Boolean {
        return ffiUpCall(defaultResult = false) {
            val event = Event.fromNative(nativeEvent)
            val result = runEventHandler(windowId, event)
            when (result) {
                EventHandlerResult.Continue -> false
                EventHandlerResult.Stop -> true
            }
        }
    }

    // called from native
    private fun pollCallbacks() {
        ffiUpCall {
            do {
                val callback = callbacksQueue.poll()
                callback?.invoke()
            } while (callback != null)
        }
    }

    private fun applicationCallbacks(): MemorySegment {
        return NativeApplicationCallbacks.allocate(arena).also { callbacks ->
            NativeApplicationCallbacks.event_handler(callbacks, NativeEventHandler.allocate(::onEvent, arena))
        }
    }

    override fun close() {
        ptr?.let { appPtr ->
            ffiDownCall {
                desktop_win32_h.application_drop(appPtr)
            }
        }
        ptr = null
        arena.close()
    }
}
