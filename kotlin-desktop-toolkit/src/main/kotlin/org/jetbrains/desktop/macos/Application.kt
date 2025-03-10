package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.macos.generated.NativeApplicationConfig
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout

public enum class EventHandlerResult {
    Continue,
    Stop,
}

public typealias EventHandler = (Event) -> EventHandlerResult
public typealias TextOperationHandler = (TextOperation) -> Boolean

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

    public lateinit var screens: AllScreens

    public fun init(applicationConfig: ApplicationConfig = ApplicationConfig()) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_macos_h.application_init(applicationConfig.toNative(arena), applicationCallbacks())
            }
        }
    }

    public fun runEventLoop() {
        ffiDownCall {
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

    // called from native
    private fun onDidChangeScreenParameters() {
        Logger.info { "onDidChangeScreenParameters" }
        screens = Screen.allScreens()
    }

    // called from native
    private fun onDidFinishLaunching() {
        Logger.info { "onDidFinishLaunching" }
        screens = Screen.allScreens()
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
        NativeApplicationCallbacks.on_did_change_screen_parameters(
            callbacks,
            NativeApplicationCallbacks.on_did_change_screen_parameters.allocate(::onDidChangeScreenParameters, arena),
        )
        NativeApplicationCallbacks.on_did_finish_launching(
            callbacks,
            NativeApplicationCallbacks.on_did_finish_launching.allocate(::onDidFinishLaunching, arena)
        )
        return callbacks
    }
}
