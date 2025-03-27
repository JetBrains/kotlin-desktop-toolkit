package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.linux.generated.NativeScreenInfo
import org.jetbrains.desktop.linux.generated.NativeScreenInfoArray
import org.jetbrains.desktop.linux.generated.NativeWindowParams
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import org.jetbrains.desktop.linux.generated.desktop_linux_h as desktop_h

public enum class EventHandlerResult {
    Continue,
    Stop,
}

public typealias EventHandler = (Event) -> EventHandlerResult

public class CustomTitlebarParams()

public data class WindowParams(
    val width: Int = 640,
    val height: Int = 480,
    val title: String = "Window",
    val appId: String = "org.jetbrains.desktop.linux.skikoSample1",
    val forceClientSideDecoration: Boolean = false,
) {
    internal fun toNative(arena: Arena): MemorySegment {
        val nativeWindowParams = NativeWindowParams.allocate(arena)
        NativeWindowParams.width(nativeWindowParams, width)
        NativeWindowParams.height(nativeWindowParams, height)
        NativeWindowParams.title(nativeWindowParams, arena.allocateUtf8String(title))
        NativeWindowParams.app_id(nativeWindowParams, arena.allocateUtf8String(appId))
        NativeWindowParams.force_client_side_decoration(nativeWindowParams, forceClientSideDecoration)
        return nativeWindowParams
    }
}

public data class ApplicationConfig(val onXdgDesktopSettingsChange: (XdgDesktopSetting) -> Unit)

public class Application() {
    private var applicationConfig: ApplicationConfig? = null

    init {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                appPtr = desktop_h.application_init(applicationCallbacks())
            }
        }
    }

    public lateinit var screens: AllScreens
    private var appPtr: MemorySegment? = null

    public fun runEventLoop(applicationConfig: ApplicationConfig) {
        this.applicationConfig = applicationConfig
        ffiDownCall {
            desktop_h.application_run_event_loop(appPtr!!)
        }
    }

    public fun stopEventLoop() {
        ffiDownCall {
            desktop_h.application_stop_event_loop(appPtr!!)
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

    private fun onNativeXdgSettingsChanged(s: MemorySegment) {
        applicationConfig?.onXdgDesktopSettingsChange(XdgDesktopSetting.fromNative(s))
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
        NativeApplicationCallbacks.on_display_configuration_change(
            callbacks,
            NativeApplicationCallbacks.on_display_configuration_change.allocate({
                screens = allScreens()
            }, arena),
        )
        NativeApplicationCallbacks.on_xdg_desktop_settings_change(
            callbacks,
            NativeApplicationCallbacks.on_xdg_desktop_settings_change.allocate(::onNativeXdgSettingsChanged, arena),
        )
        return callbacks
    }

    public fun createWindow(eventHandler: EventHandler, params: WindowParams): Window {
        return Window(appPtr!!, eventHandler, params)
    }

    public fun allScreens(): AllScreens {
        return Arena.ofConfined().use { arena ->
            val screenInfoArray = ffiDownCall { desktop_h.screen_list(arena, appPtr!!) }
            val screens = mutableListOf<Screen>()
            try {
                val ptr = NativeScreenInfoArray.ptr(screenInfoArray)
                val len = NativeScreenInfoArray.len(screenInfoArray)

                for (i in 0 until len) {
                    screens.add(Screen.fromNative(NativeScreenInfo.asSlice(ptr, i)))
                }
            } finally {
                ffiDownCall { desktop_h.screen_list_drop(screenInfoArray) }
            }
            AllScreens(screens)
        }
    }
}
