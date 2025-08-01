package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.win32.generated.NativeEventHandler
import org.jetbrains.desktop.win32.generated.desktop_windows_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object Application {
    private var appPtr: MemorySegment? = null
    private var eventHandler: EventHandler? = null

    public fun init() {
        ffiDownCall {
            appPtr = Arena.ofConfined().use { arena ->
                desktop_windows_h.application_init(applicationCallbacks())
            }
        }
    }

    public fun runEventLoop(eventHandler: EventHandler) {
        ffiDownCall {
            this.eventHandler = eventHandler
            desktop_windows_h.application_run_event_loop(appPtr!!)
        }
    }

    public fun stopEventLoop() {
        ffiDownCall {
            desktop_windows_h.application_stop_event_loop(appPtr!!)
        }
    }

    public fun createWindow(params: WindowParams): Window {
        return Window.create(appPtr!!, params)
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

    private fun applicationCallbacks(): MemorySegment {
        val arena = Arena.global()
        val callbacks = NativeApplicationCallbacks.allocate(arena)
        NativeApplicationCallbacks.event_handler(callbacks, NativeEventHandler.allocate(::onEvent, arena))
        return callbacks
    }
}
