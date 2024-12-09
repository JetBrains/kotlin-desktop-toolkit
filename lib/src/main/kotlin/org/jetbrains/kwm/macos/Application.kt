package org.jetbrains.kwm.macos

import org.jetbrains.kwm.IApplication
import org.jetbrains.kwm.macos.generated.ApplicationCallbacks
import org.jetbrains.kwm.macos.generated.ApplicationConfig
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.CountDownLatch
import kotlin.concurrent.thread

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
            kwm_macos_h.application_init(config.toNative(arena), applicationCallbacks())
        }
        Runtime.getRuntime().addShutdownHook(Thread {
            var x = 0
            while (true) {
                x += 1
                println("x: $x")
                Thread.sleep(500)
            }
        })
        Runtime.getRuntime().addShutdownHook(Thread({
            GrandCentralDispatch.dispatchOnMainSync {
                isReadyForTermination = true
                requestTermination()
            }
        }, "NSApplication shutdown"))
    }

    // This method never returns
    override fun runEventLoop() {
        kwm_macos_h.application_run_event_loop()
    }

    fun requestTermination() {
        kwm_macos_h.application_request_termination()
    }

    @Volatile
    private var isReadyForTermination = false

    private fun onShouldTerminate(): Boolean {
        println("Should terminate?")
        return isReadyForTermination
    }

    private fun onWillTerminate() {
        println("Will terminate!")
        Thread.sleep(5  * 1000)
    }

    private fun applicationCallbacks(): MemorySegment {
        val callbacks = ApplicationCallbacks.allocate(Arena.global())
        ApplicationCallbacks.on_should_terminate(callbacks, ApplicationCallbacks.on_should_terminate.allocate(::onShouldTerminate, Arena.global()))
        ApplicationCallbacks.on_will_terminate(callbacks, ApplicationCallbacks.on_will_terminate.allocate(::onWillTerminate, Arena.global()))
        return callbacks
    }
}