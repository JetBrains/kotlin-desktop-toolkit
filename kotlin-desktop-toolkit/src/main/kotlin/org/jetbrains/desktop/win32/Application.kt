package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.win32.generated.NativeEventHandler
import org.jetbrains.desktop.win32.generated.`application_dispatcher_invoke$callback`
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.ConcurrentLinkedQueue

public class Application : AutoCloseable {
    private val arena: Arena = Arena.ofShared()

    private val appPtr: MemorySegment
    private var eventHandler: EventHandler? = null

    private val callbacksQueue = ConcurrentLinkedQueue<() -> Unit>()
    private val callback: MemorySegment

    init {
        callback = `application_dispatcher_invoke$callback`.allocate(::pollCallbacks, arena)
        appPtr = ffiDownCall {
            desktop_win32_h.application_init(applicationCallbacks())
        }
    }

    public fun invokeOnDispatcher(body: () -> Unit) {
        if (callbacksQueue.offer(body)) {
            desktop_win32_h.application_dispatcher_invoke(appPtr, callback)
        }
    }

    public fun runEventLoop(onStartup: () -> Unit, eventHandler: EventHandler) {
        this.eventHandler = eventHandler
        invokeOnDispatcher(onStartup)
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
        arena.close()
    }
}
