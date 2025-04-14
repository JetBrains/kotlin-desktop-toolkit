package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.macos.generated.NativeApplicationConfig
import org.jetbrains.desktop.macos.generated.NativeEventHandler
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout

public enum class EventHandlerResult {
    Continue,
    Stop,
}

public typealias EventHandler = (Event) -> EventHandlerResult

public object Application {
    public data class ApplicationConfig(
        val disableDictationMenuItem: Boolean = false,
        val disableCharacterPaletteMenuItem: Boolean = false,
    ) {
        internal fun toNative(arena: Arena): MemorySegment {
            val config = NativeApplicationConfig.allocate(arena)
            NativeApplicationConfig.disable_dictation_menu_item(config, disableDictationMenuItem)
            NativeApplicationConfig.disable_character_palette_menu_item(config, disableCharacterPaletteMenuItem)
            return config
        }
    }

    private var eventHandler: EventHandler? = null
    public lateinit var screens: AllScreens

    public fun init(applicationConfig: ApplicationConfig = ApplicationConfig()) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.application_init(applicationConfig.toNative(arena), applicationCallbacks())
            }
        }
    }

    public fun runEventLoop(eventHandler: EventHandler) {
        ffiDownCall {
            this.eventHandler = eventHandler
            desktop_macos_h.application_run_event_loop()
        }
    }

    public fun stopEventLoop() {
        ffiDownCall {
            desktop_macos_h.application_stop_event_loop()
        }
    }

    public fun requestTermination() {
        ffiDownCall {
            desktop_macos_h.application_request_termination()
        }
    }

    public val name: String
        get() {
            val name = ffiDownCall { desktop_macos_h.application_get_name() }
            return try {
                name.getUtf8String(0)
            } finally {
                ffiDownCall { desktop_macos_h.string_drop(name) }
            }
        }

    public val appearance: Appearance
        get() {
            return ffiDownCall {
                Appearance.fromNative(desktop_macos_h.application_get_appearance())
            }
        }

    public fun hide() {
        ffiDownCall {
            desktop_macos_h.application_hide()
        }
    }

    public fun hideOtherApplications() {
        ffiDownCall {
            desktop_macos_h.application_hide_other_applications()
        }
    }

    public fun unhideAllApplications() {
        ffiDownCall {
            desktop_macos_h.application_unhide_all_applications()
        }
    }

    public fun setDockIcon(icon: ByteArray) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                val segment = arena.allocateArray(ValueLayout.JAVA_BYTE, *icon)
                desktop_macos_h.application_set_dock_icon(segment, segment.byteSize())
            }
        }
    }

    public fun orderFrontCharactersPalette() {
        ffiDownCall {
            desktop_macos_h.application_order_front_character_palete()
        }
    }

    public fun setQuitHandler(isSafeToQuit: () -> Boolean) {
        this.isSafeToQuit = isSafeToQuit
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
}
