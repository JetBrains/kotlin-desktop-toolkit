package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.ApplicationCallbacks
import org.jetbrains.kwm.macos.generated.ApplicationConfig
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

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

    fun init(config: Config = Config()) {
        Arena.ofConfined().use { arena ->
            kwm_macos_h.application_init(config.toNative(arena), applicationCallbacks())
        }
    }

    fun runEventLoop() {
        kwm_macos_h.application_run_event_loop()
    }

    fun stopEventLoop() {
        kwm_macos_h.application_stop_event_loop()
    }

    fun requestTermination() {
        kwm_macos_h.application_request_termination()
    }

    private fun onShouldTerminate(): Boolean {
        // todo send event to request user interaction?
        return false
    }

    private fun onWillTerminate() {
        // This method will never be executed because
        // the application halt is performed immediately after that
        // which means that JVM shutdown hooks might be interupted
    }

    private fun applicationCallbacks(): MemorySegment {
        val callbacks = ApplicationCallbacks.allocate(Arena.global())
        ApplicationCallbacks.on_should_terminate(callbacks, ApplicationCallbacks.on_should_terminate.allocate(::onShouldTerminate, Arena.global()))
        ApplicationCallbacks.on_will_terminate(callbacks, ApplicationCallbacks.on_will_terminate.allocate(::onWillTerminate, Arena.global()))
        return callbacks
    }
}