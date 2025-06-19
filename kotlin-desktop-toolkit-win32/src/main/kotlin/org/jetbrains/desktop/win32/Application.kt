package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.ffiDownCall
import org.jetbrains.desktop.win32.generated.desktop_windows_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

//public enum class EventHandlerResult {
//    Continue,
//    Stop,
//}

//public typealias EventHandler = (Event) -> EventHandlerResult

public object Application {
    private var appPtr: MemorySegment? = null
    //private var eventHandler: EventHandler? = null
    //public lateinit var screens: AllScreens

    public fun init(/*applicationConfig: ApplicationConfig = ApplicationConfig()*/) {
        ffiDownCall {
            appPtr = Arena.ofConfined().use { arena ->
                desktop_windows_h.application_init(/*applicationConfig.toNative(arena), applicationCallbacks()*/)
            }
        }
    }

    public fun runEventLoop(/*eventHandler: EventHandler*/) {
        ffiDownCall {
            // this.eventHandler = eventHandler
            desktop_windows_h.application_run_event_loop()
        }
    }

    public fun stopEventLoop() {
        ffiDownCall {
            desktop_windows_h.application_stop_event_loop()
        }
    }

    public fun createWindow(params: WindowParams): Window {
        return Window.create(appPtr!!, params)
    }

    /*
    public fun requestTermination() {
        ffiDownCall {
            desktop_windows_h.application_request_termination()
        }
    }

    public val name: String
        get() {
            val name = ffiDownCall { desktop_windows_h.application_get_name() }
            return try {
                name.getUtf8String(0)
            } finally {
                ffiDownCall { desktop_windows_h.string_drop(name) }
            }
        }

    public val appearance: Appearance
        get() {
            return ffiDownCall {
                Appearance.fromNative(desktop_windows_h.application_get_appearance())
            }
        }

    public fun hide() {
        ffiDownCall {
            desktop_windows_h.application_hide()
        }
    }

    public fun hideOtherApplications() {
        ffiDownCall {
            desktop_windows_h.application_hide_other_applications()
        }
    }

    public fun unhideAllApplications() {
        ffiDownCall {
            desktop_windows_h.application_unhide_all_applications()
        }
    }

    public fun setDockIcon(icon: ByteArray) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                val segment = arena.allocateArray(ValueLayout.JAVA_BYTE, *icon)
                desktop_windows_h.application_set_dock_icon(segment, segment.byteSize())
            }
        }
    }

    public fun orderFrontCharactersPalette() {
        ffiDownCall {
            desktop_windows_h.application_order_front_character_palete()
        }
    }

    public fun setQuitHandler(isSafeToQuit: () -> Boolean) {
        this.isSafeToQuit = isSafeToQuit
    }

    public fun openURL(url: String) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_windows_h.application_open_url(arena.allocateUtf8String(url))
            }
        }
    }

    private var isSafeToQuit: () -> Boolean = { true }

    // called from native
    private fun onShouldTerminate(): Boolean {
        Logger.info { "onShouldTerminate" }
        return ffiUpCall(defaultResult = false) {
            isSafeToQuit()
        }
    }

    // called from native
    private fun onWillTerminate() {
        Logger.info { "onWillTerminate" }
        // This method will never be executed because
        // the application halt is performed immediately after that
        // which means that JVM shutdown hooks might be interupted
        ffiUpCall {
        }
    }

    private fun runEventHandler(event: Event): EventHandlerResult {
        return eventHandler?.let { eventHandler ->
            eventHandler(event)
        } ?: run {
            Logger.warn { "eventHandler is null; event: $event was ignored!" }
            EventHandlerResult.Continue
        }
    }

    // called from native
    private fun onEvent(nativeEvent: MemorySegment): Boolean {
        return ffiUpCall(defaultResult = false) {
            val event = Event.fromNative(nativeEvent)
            when (event) {
                is Event.ApplicationDidFinishLaunching -> {
                    screens = Screen.allScreens()
                }
                is Event.DisplayConfigurationChange -> {
                    screens = Screen.allScreens()
                }
                else -> {}
            }
            val result = runEventHandler(event)
            when (result) {
                EventHandlerResult.Continue -> false
                EventHandlerResult.Stop -> true
            }
        }
    }

    private fun applicationCallbacks(): MemorySegment {
        val arena = Arena.global()
        val callbacks = NativeApplicationCallbacks.allocate(arena)
        NativeApplicationCallbacks.on_should_terminate(
            callbacks,
            NativeApplicationCallbacks.on_should_terminate.allocate(::onShouldTerminate, arena),
        )
        NativeApplicationCallbacks.on_will_terminate(
            callbacks,
            NativeApplicationCallbacks.on_will_terminate.allocate(::onWillTerminate, arena),
        )
        NativeApplicationCallbacks.event_handler(callbacks, NativeEventHandler.allocate(::onEvent, arena))
        return callbacks
    }
    */
}
