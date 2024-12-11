package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.EventHandler as NativeEventHandler
import org.jetbrains.kwm.macos.generated.ApplicationCallbacks
import org.jetbrains.kwm.macos.generated.ApplicationConfig
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

enum class EventHandlerResult {
    Skipped,
    Handled
}

typealias EventHandler = (Event) -> EventHandlerResult

object Application {
    data class Config(val disableDictationMenuItem: Boolean = false,
                      val disableCharacterPaletteMenuItem: Boolean = false) {
        internal fun toNative(arena: Arena): MemorySegment? {
            val config = ApplicationConfig.allocate(arena)
            ApplicationConfig.disable_dictation_menu_item(config, disableDictationMenuItem)
            ApplicationConfig.disable_character_palette_menu_item(config, disableCharacterPaletteMenuItem)
            return config
        }
    }

    var eventHandler: EventHandler? = null

    fun init(config: Config = Config()) {
        Arena.ofConfined().use { arena ->
            kwm_macos_h.application_init(config.toNative(arena), applicationCallbacks())
        }
    }

    fun runEventLoop(eventHandler: EventHandler = { EventHandlerResult.Skipped }) {
        this.eventHandler = eventHandler
        kwm_macos_h.application_run_event_loop()
    }

    fun stopEventLoop() {
        kwm_macos_h.application_stop_event_loop()
    }

    fun requestTermination() {
        kwm_macos_h.application_request_termination()
    }

    // called from native
    private fun onShouldTerminate(): Boolean {
        // todo send event to request user interaction?
        return false
    }

    // called from native
    private fun onWillTerminate() {
        // This method will never be executed because
        // the application halt is performed immediately after that
        // which means that JVM shutdown hooks might be interupted
    }

    // called from native
    private fun onEvent(nativeEvent: MemorySegment): Boolean {
        val event = Event.fromNative(nativeEvent)
        return eventHandler?.let { eventHandler ->
            when (eventHandler(event)) {
                EventHandlerResult.Skipped -> false
                EventHandlerResult.Handled -> true
            }
        } ?: run {
            // todo remove with proper logging
            println("eventHandler is null event: $event was ignored!")
            false
        }
    }

    private fun applicationCallbacks(): MemorySegment {
        val arena = Arena.global()
        val callbacks = ApplicationCallbacks.allocate(arena)
        ApplicationCallbacks.on_should_terminate(callbacks, ApplicationCallbacks.on_should_terminate.allocate(::onShouldTerminate, arena))
        ApplicationCallbacks.on_will_terminate(callbacks, ApplicationCallbacks.on_will_terminate.allocate(::onWillTerminate, arena))
        ApplicationCallbacks.event_handler(callbacks, NativeEventHandler.allocate(::onEvent, arena))
        return callbacks
    }
}