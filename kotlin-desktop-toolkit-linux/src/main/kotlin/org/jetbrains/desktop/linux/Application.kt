package org.jetbrains.desktop.linux

import org.jetbrains.desktop.linux.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.linux.generated.NativeEventHandler
import org.jetbrains.desktop.linux.generated.NativeGetEglProcFuncData
import org.jetbrains.desktop.linux.generated.NativeScreenInfo
import org.jetbrains.desktop.linux.generated.NativeScreenInfoArray
import org.jetbrains.desktop.linux.generated.NativeWindowParams
import org.jetbrains.desktop.linux.generated.`application_run_on_event_loop_async$f`
import org.jetbrains.desktop.linux.generated.desktop_linux_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.ConcurrentLinkedQueue

public enum class EventHandlerResult {
    Continue,
    Stop,
}

public typealias EventHandler = (Event, WindowId) -> EventHandlerResult

public class CustomTitlebarParams()

public data class WindowParams(
    val windowId: WindowId,
    val appId: String,
    val title: String,
    val size: LogicalSize? = null,
    val forceClientSideDecoration: Boolean = false,
    val forceSoftwareRendering: Boolean = false,
) {
    internal fun toNative(arena: Arena): MemorySegment {
        val nativeWindowParams = NativeWindowParams.allocate(arena)
        NativeWindowParams.size(nativeWindowParams, (size ?: LogicalSize(0f, 0f)).toNative(arena))
        NativeWindowParams.title(nativeWindowParams, arena.allocateUtf8String(title))
        NativeWindowParams.app_id(nativeWindowParams, arena.allocateUtf8String(appId))
        NativeWindowParams.force_client_side_decoration(nativeWindowParams, forceClientSideDecoration)
        NativeWindowParams.force_software_rendering(nativeWindowParams, forceSoftwareRendering)
        NativeWindowParams.window_id(nativeWindowParams, windowId)
        return nativeWindowParams
    }
}

public enum class DataSource {
    Clipboard,
    DragAndDrop,
    ;

    internal companion object
}

public data class ApplicationConfig(
    val onApplicationStarted: () -> Unit,
    val onXdgDesktopSettingsChange: (XdgDesktopSetting) -> Unit,
    val eventHandler: EventHandler,
    val getDragAndDropSupportedMimeTypes: (DragAndDropQueryData) -> List<String>,
    val getDataTransferData: (DataSource, String) -> ByteArray?,
    val onDataTransferCancelled: (DataSource) -> Unit,
)

public class Application() : AutoCloseable {
    private var applicationConfig: ApplicationConfig? = null

    private val runOnEventLoopAsyncQueue = ConcurrentLinkedQueue<() -> Unit>()
    private val runOnEventLoopAsyncFunc: MemorySegment = `application_run_on_event_loop_async$f`.allocate({
        ffiUpCall {
            runOnEventLoopAsyncQueue.poll().invoke()
        }
    }, Arena.global())

    public lateinit var screens: AllScreens
    private val mimeTypeReturnCache: HashMap<List<String>, MemorySegment> = hashMapOf()
    private var appPtr: MemorySegment? = null

    init {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                appPtr = desktop_linux_h.application_init(applicationCallbacks())
            }
        }
    }

    override fun toString(): String {
        return "${javaClass.typeName}(ptr=0x${appPtr?.address()?.toString(16)})"
    }

    // called from native
    private fun onEvent(nativeEvent: MemorySegment, windowId: WindowId): Boolean {
        val event = Event.fromNative(nativeEvent)
        return ffiUpCall(defaultResult = false) {
            val result = applicationConfig?.eventHandler(event, windowId)
            when (result) {
                EventHandlerResult.Continue -> false
                EventHandlerResult.Stop -> true
                null -> false
            }
        }
    }

    private fun mimeTypesToNative(mimeTypes: List<String>): MemorySegment {
        return mimeTypeReturnCache.getOrPut(mimeTypes) {
            Arena.global().allocateUtf8String(mimeTypes.joinToString(","))
        }
    }

    // called from native
    private fun onGetDataTransferData(nativeDataSource: Int, nativeMimeType: MemorySegment): MemorySegment {
        val dataSource = DataSource.fromNative(nativeDataSource)
        val mimeType = nativeMimeType.getUtf8String(0)
        return ffiUpCall(defaultResult = MemorySegment.NULL) {
            val result = applicationConfig?.getDataTransferData(dataSource, mimeType)
            val arena = Arena.ofConfined()
            val nativeResult = result.toNative(arena)
            nativeResult
        }
    }

    // called from native
    private fun onGetDragAndDropSupportedMimeTypes(nativeQueryData: MemorySegment): MemorySegment {
        val queryData = DragAndDropQueryData.fromNative(nativeQueryData)
        return ffiUpCall(defaultResult = MemorySegment.NULL) {
            val result = applicationConfig?.getDragAndDropSupportedMimeTypes(queryData) ?: emptyList()
            mimeTypesToNative(result)
        }
    }

    // called from native
    private fun onDataTransferCancelled(nativeDataSource: Int) {
        val dataSource = DataSource.fromNative(nativeDataSource)
        ffiUpCall {
            applicationConfig?.onDataTransferCancelled(dataSource)
        }
    }

    public fun runEventLoop(applicationConfig: ApplicationConfig) {
        this.applicationConfig = applicationConfig
        ffiDownCall {
            desktop_linux_h.application_run_event_loop(appPtr!!)
        }
    }

    public fun stopEventLoop() {
        ffiDownCall {
            desktop_linux_h.application_stop_event_loop(appPtr!!)
        }
    }

    public override fun close() {
        ffiDownCall {
            desktop_linux_h.application_shutdown(appPtr!!)
        }
    }

    public fun setQuitHandler(isSafeToQuit: () -> Boolean) {
        this.isSafeToQuit = isSafeToQuit
    }

    public fun openURL(url: String) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_linux_h.application_open_url(arena.allocateUtf8String(url))
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
        // the application halt is performed immediately after that,
        // which means that JVM shutdown hooks might be interrupted
        ffiUpCall {
        }
    }

    private fun onApplicationStarted() {
        applicationConfig?.onApplicationStarted()
    }

    private fun onNativeXdgSettingsChanged(s: MemorySegment) {
        applicationConfig?.onXdgDesktopSettingsChange(XdgDesktopSetting.fromNative(s))
    }

    private fun applicationCallbacks(): MemorySegment {
        val arena = Arena.global()
        val callbacks = NativeApplicationCallbacks.allocate(arena)
        NativeApplicationCallbacks.on_application_started(
            callbacks,
            NativeApplicationCallbacks.on_application_started.allocate(::onApplicationStarted, arena),
        )
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
        NativeApplicationCallbacks.event_handler(callbacks, NativeEventHandler.allocate(::onEvent, arena))
        NativeApplicationCallbacks.get_drag_and_drop_supported_mime_types(
            callbacks,
            NativeApplicationCallbacks.get_drag_and_drop_supported_mime_types.allocate(::onGetDragAndDropSupportedMimeTypes, arena),
        )
        NativeApplicationCallbacks.get_data_transfer_data(
            callbacks,
            NativeApplicationCallbacks.get_data_transfer_data.allocate(::onGetDataTransferData, arena),
        )
        NativeApplicationCallbacks.on_data_transfer_cancelled(
            callbacks,
            NativeApplicationCallbacks.on_data_transfer_cancelled.allocate(::onDataTransferCancelled, arena),
        )
        return callbacks
    }

    public fun createWindow(params: WindowParams): Window {
        return Window(appPtr!!, params)
    }

    public fun setCursorTheme(name: String, size: Int) {
        Arena.ofConfined().use { arena ->
            desktop_linux_h.application_set_cursor_theme(appPtr, arena.allocateUtf8String(name), size)
        }
    }

    public fun isEventLoopThread(): Boolean {
        return ffiDownCall {
            desktop_linux_h.application_is_event_loop_thread(appPtr)
        }
    }

    public fun runOnEventLoopAsync(f: () -> Unit) {
        ffiDownCall {
            runOnEventLoopAsyncQueue.add(f)
            desktop_linux_h.application_run_on_event_loop_async(appPtr, runOnEventLoopAsyncFunc)
        }
    }

    public fun allScreens(): AllScreens {
        return Arena.ofConfined().use { arena ->
            val screenInfoArray = ffiDownCall { desktop_linux_h.screen_list(arena, appPtr!!) }
            val screens = mutableListOf<Screen>()
            try {
                val ptr = NativeScreenInfoArray.ptr(screenInfoArray)
                val len = NativeScreenInfoArray.len(screenInfoArray)

                for (i in 0 until len) {
                    screens.add(Screen.fromNative(NativeScreenInfo.asSlice(ptr, i)))
                }
            } finally {
                ffiDownCall { desktop_linux_h.screen_list_drop(screenInfoArray) }
            }
            AllScreens(screens)
        }
    }

    public data class EglProcFunc(val fPtr: Long, val ctxPtr: Long)

    public fun getEglProcFunc(): EglProcFunc? {
        return Arena.ofConfined().use { arena ->
            val s = desktop_linux_h.application_get_egl_proc_func(arena, appPtr!!)
            val f = NativeGetEglProcFuncData.f(s)
            val ctx = NativeGetEglProcFuncData.ctx(s)
            if (ctx == MemorySegment.NULL) null else EglProcFunc(fPtr = f.address(), ctxPtr = ctx.address())
        }
    }

    /** Should be called after [Event.TextInputAvailability] reports `true`, if Text Input support is needed. */
    public fun textInputEnable(context: TextInputContext) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_linux_h.application_text_input_enable(appPtr, context.toNative(arena))
            }
        }
    }

    /** Should be called after any data in [TextInputContext] is changed, but only if [textInputEnable] was called beforehand. */
    public fun textInputUpdate(context: TextInputContext) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_linux_h.application_text_input_update(appPtr, context.toNative(arena))
            }
        }
    }

    /** Disable Text Input support, if [textInputEnable] was called beforehand. */
    public fun textInputDisable() {
        ffiDownCall {
            desktop_linux_h.application_text_input_disable(appPtr)
        }
    }

    /**
     * Indicate that there is data that other applications can fetch from clipboard, in any of the provided MIME type formats.
     * Later, [ApplicationConfig.getDataTransferData] may be called, with [DataSource.Clipboard] argument,
     * to actually get the data with the specified MIME type.
     */
    public fun clipboardPut(mimeTypes: List<String>) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.application_clipboard_put(appPtr, mimeTypesToNative(arena, mimeTypes))
            }
        }
    }
}
