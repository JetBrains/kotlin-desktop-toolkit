package org.jetbrains.kwm.macos

import org.jetbrains.kwm.IApplication
import org.jetbrains.kwm.macos.generated.ApplicationConfig
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

object Application: IApplication {
    override fun init() {
        initWithConfig(Config())
    }

    data class Config(val disableDictationMenuItem: Boolean = false,
                      val disableCharacterPaletteMenuItem: Boolean = false) {
        internal fun toNative(arena: Arena): MemorySegment? {
            val config = ApplicationConfig.allocate(arena)
            ApplicationConfig.disable_dictation_menu_item(config, disableDictationMenuItem)
            ApplicationConfig.disable_character_palette_menu_item(config, disableCharacterPaletteMenuItem)
            return config
        }
    }

    fun initWithConfig(config: Config) {
        Arena.ofConfined().use { arena ->
            kwm_macos_h.application_init(config.toNative(arena))
        }
    }

    override fun runEventLoop() {
        kwm_macos_h.application_run_event_loop()
    }

    fun createWindow(title: String, x: Float, y: Float) {
        Arena.ofConfined().use { arena ->
            val title = arena.allocateUtf8String(title)
            kwm_macos_h.application_create_window(title, x, y)
        }
    }
}