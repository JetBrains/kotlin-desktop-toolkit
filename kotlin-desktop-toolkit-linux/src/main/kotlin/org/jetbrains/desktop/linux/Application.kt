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
    val isResizable: Boolean = true,
    val isClosable: Boolean = true,
    val isMiniaturizable: Boolean = true,
    val isFullScreenAllowed: Boolean = true,
    val customTitlebar: CustomTitlebarParams? = null,
) {
    internal fun toNative(arena: Arena): MemorySegment {
        val nativeWindowParams = NativeWindowParams.allocate(arena)
//            NativeWindowParams.origin(nativeWindowParams, origin.toNative(arena))
        NativeWindowParams.width(nativeWindowParams, width)
        NativeWindowParams.height(nativeWindowParams, height)
//            NativeWindowParams.title(nativeWindowParams, arena.allocateUtf8String(title))
//
//            NativeWindowParams.is_resizable(nativeWindowParams, isResizable)
//            NativeWindowParams.is_closable(nativeWindowParams, isClosable)
//            NativeWindowParams.is_miniaturizable(nativeWindowParams, isMiniaturizable)
//            NativeWindowParams.is_full_screen_allowed(nativeWindowParams, isFullScreenAllowed)
//            NativeWindowParams.use_custom_titlebar(nativeWindowParams, useCustomTitlebar)
//            NativeWindowParams.titlebar_height(nativeWindowParams, titlebarHeight)
        return nativeWindowParams
    }
}

public class ApplicationConfig()

public class Application(applicationConfig: ApplicationConfig = ApplicationConfig()) {
    init {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                appPtr = desktop_h.application_init(applicationCallbacks())
            }
        }
    }

    public lateinit var screens: AllScreens
    private var appPtr: MemorySegment? = null

    public fun runEventLoop() {
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
        return callbacks
    }

    public fun createWindow(eventHandler: EventHandler, params: WindowParams): Window {
        return Window(appPtr!!, eventHandler, params)
    }

    public fun createWindow(
        eventHandler: EventHandler,
        width: Int = 640,
        height: Int = 480,
        title: String = "Window",
        isResizable: Boolean = true,
        isClosable: Boolean = true,
        isMiniaturizable: Boolean = true,
        isFullScreenAllowed: Boolean = true,
        customTitlebar: CustomTitlebarParams? = null,
    ): Window {
        return createWindow(
            eventHandler,
            WindowParams(
                width,
                height,
                title,
                isResizable,
                isClosable,
                isMiniaturizable,
                isFullScreenAllowed,
                customTitlebar,
            ),
        )
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
