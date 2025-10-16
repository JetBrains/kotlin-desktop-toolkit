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

public typealias EventHandler = (Event) -> EventHandlerResult

public class CustomTitlebarParams

public enum class RenderingMode {
    Auto,
    Software,
    EGL,
}

public data class WindowParams(
    val windowId: WindowId,
    val appId: String,
    val title: String,
    val size: LogicalSize? = null,
    val preferClientSideDecoration: Boolean = false,
    val renderingMode: RenderingMode = RenderingMode.Auto,
) {
    internal fun toNative(arena: Arena): MemorySegment {
        val nativeWindowParams = NativeWindowParams.allocate(arena)
        NativeWindowParams.size(nativeWindowParams, (size ?: LogicalSize(0, 0)).toNative(arena))
        NativeWindowParams.title(nativeWindowParams, arena.allocateUtf8String(title))
        NativeWindowParams.app_id(nativeWindowParams, arena.allocateUtf8String(appId))
        NativeWindowParams.prefer_client_side_decoration(nativeWindowParams, preferClientSideDecoration)
        NativeWindowParams.rendering_mode(nativeWindowParams, renderingMode.toNative())
        NativeWindowParams.window_id(nativeWindowParams, windowId)
        return nativeWindowParams
    }
}

public enum class DataSource {
    Clipboard,
    DragAndDrop,
    PrimarySelection,
    ;

    internal companion object
}

public data class ApplicationConfig(
    val eventHandler: EventHandler,
    val queryDragAndDropTarget: (DragAndDropQueryData) -> DragAndDropQueryResponse,
    val getDataTransferData: (DataSource, String) -> ByteArray?,
)

public class ShowNotificationParams(
    /** User-visible string to display as the title.
     * This should be a short string, if it doesn’t fit the UI, it may be truncated to fit on a single line.
     */
    public val title: String,

    /** User-visible string to display as the body.
     * This can be a long string, but if it doesn’t fit the UI, it may be wrapped or/and truncated.
     */
    public val body: String,

    /** The path to a sound file to play when the notification pops up.
     * The mandatory supported sound file formats are WAV/PCM 8-48kHz, 8/16bits, and OGG/Vorbis I.
     */
    public val soundFilePath: String?,
)

public class Application : AutoCloseable {
    private var applicationConfig: ApplicationConfig? = null

    private val runOnEventLoopAsyncQueue = ConcurrentLinkedQueue<() -> Unit>()
    private val runOnEventLoopAsyncFunc: MemorySegment = `application_run_on_event_loop_async$f`.allocate({
        ffiUpCall {
            runOnEventLoopAsyncQueue.poll().invoke()
        }
    }, Arena.global())

    private val mimeTypeReturnCache: HashMap<List<String>, MemorySegment> = hashMapOf()
    private var appPtr: MemorySegment? = null

    init {
        ffiDownCall {
            appPtr = desktop_linux_h.application_init(applicationCallbacks())
        }
    }

    override fun toString(): String {
        return "${javaClass.typeName}(ptr=0x${appPtr?.address()?.toString(16)})"
    }

    // called from native
    private fun onEvent(nativeEvent: MemorySegment): Boolean {
        val event = Event.fromNative(nativeEvent, this)
        return ffiUpCall(defaultResult = false) {
            val result = applicationConfig?.eventHandler(event)
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
        val result = applicationConfig?.getDataTransferData(dataSource, mimeType)
        return result.toNative()
    }

    // called from native
    private fun onQueryDragAndDropTarget(nativeQueryData: MemorySegment): MemorySegment {
        val queryData = DragAndDropQueryData.fromNative(nativeQueryData)
        val result = applicationConfig?.queryDragAndDropTarget(queryData) ?: DragAndDropQueryResponse(
            supportedActionsPerMime = emptyList(),
        )
        return result.toNative()
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

    public fun openURL(url: String) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_linux_h.application_open_url(appPtr!!, arena.allocateUtf8String(url))
            }
        }
    }

    private fun applicationCallbacks(): MemorySegment {
        val arena = Arena.global()
        val callbacks = NativeApplicationCallbacks.allocate(arena)
        NativeApplicationCallbacks.event_handler(callbacks, NativeEventHandler.allocate(::onEvent, arena))
        NativeApplicationCallbacks.query_drag_and_drop_target(
            callbacks,
            NativeApplicationCallbacks.query_drag_and_drop_target.allocate(::onQueryDragAndDropTarget, arena),
        )
        NativeApplicationCallbacks.get_data_transfer_data(
            callbacks,
            NativeApplicationCallbacks.get_data_transfer_data.allocate(::onGetDataTransferData, arena),
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

    public data class EglProcFunc(
        val fPtr: Long,
        val ctxPtr: Long,
    )

    public fun getEglProcFunc(): EglProcFunc? {
        return Arena.ofConfined().use { arena ->
            val s = desktop_linux_h.application_get_egl_proc_func(arena)
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

    /** Will produce [Event.DataTransfer] event if there is clipboard content. */
    public fun clipboardPaste(serial: Int, supportedMimeTypes: List<String>): Boolean {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.application_clipboard_paste(appPtr, serial, mimeTypesToNative(arena, supportedMimeTypes))
            }
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

    public fun clipboardGetAvailableMimeTypes(): List<String> {
        val csvMimetypes = ffiDownCall { desktop_linux_h.application_clipboard_get_available_mimetypes(appPtr) }
        return try {
            csvMimetypes.getUtf8String(0).split(",")
        } finally {
            ffiDownCall { desktop_linux_h.string_drop(csvMimetypes) }
        }
    }

    /** Will produce [Event.DataTransfer] event if there is primary selection content. */
    public fun primarySelectionPaste(serial: Int, supportedMimeTypes: List<String>): Boolean {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.application_primary_selection_paste(appPtr, serial, mimeTypesToNative(arena, supportedMimeTypes))
            }
        }
    }

    /**
     * Indicate that there is data that other applications can fetch from primary selection, in any of the provided MIME type formats.
     * Later, [ApplicationConfig.getDataTransferData] may be called, with [DataSource.PrimarySelection] argument,
     * to actually get the data with the specified MIME type.
     */
    public fun primarySelectionPut(mimeTypes: List<String>) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_linux_h.application_primary_selection_put(appPtr, mimeTypesToNative(arena, mimeTypes))
            }
        }
    }

    public fun primarySelectionGetAvailableMimeTypes(): List<String> {
        val csvMimetypes = ffiDownCall { desktop_linux_h.application_primary_selection_get_available_mimetypes(appPtr) }
        return try {
            csvMimetypes.getUtf8String(0).split(",")
        } finally {
            ffiDownCall { desktop_linux_h.string_drop(csvMimetypes) }
        }
    }

    @Deprecated("Use `Window.requestInternalActivationToken` instead")
    public fun requestInternalActivationToken(sourceWindowId: WindowId): RequestId? {
        return ffiDownCall {
            val rawRequestId = desktop_linux_h.window_request_internal_activation_token(appPtr, sourceWindowId)
            if (rawRequestId == 0) {
                null
            } else {
                RequestId(rawRequestId)
            }
        }
    }

    public fun requestShowNotification(params: ShowNotificationParams): RequestId? {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val title = arena.allocateUtf8String(params.title)
                val body = arena.allocateUtf8String(params.body)
                val soundFilePath = params.soundFilePath?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL
                val rawRequestId = desktop_linux_h.application_request_show_notification(appPtr, title, body, soundFilePath)
                if (rawRequestId == 0) {
                    null
                } else {
                    RequestId(rawRequestId)
                }
            }
        }
    }

    public fun closeNotification(notificationId: UInt) {
        ffiDownCall {
            desktop_linux_h.application_close_notification(appPtr, notificationId.toInt())
        }
    }
}
