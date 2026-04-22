package org.jetbrains.desktop.gtk

import org.jetbrains.desktop.gtk.generated.NativeApplicationCallbacks
import org.jetbrains.desktop.gtk.generated.NativeEventHandler
import org.jetbrains.desktop.gtk.generated.NativeFfiApplicationWantsToTerminate
import org.jetbrains.desktop.gtk.generated.NativeFfiObjDealloc
import org.jetbrains.desktop.gtk.generated.NativeFfiQueryDragAndDropTarget
import org.jetbrains.desktop.gtk.generated.NativeFfiRetrieveSurroundingText
import org.jetbrains.desktop.gtk.generated.NativeFfiTransferDataGetter
import org.jetbrains.desktop.gtk.generated.NativeFfiWindowCloseRequest
import org.jetbrains.desktop.gtk.generated.NativeGetGlProcFuncData
import org.jetbrains.desktop.gtk.generated.NativeScreenInfo
import org.jetbrains.desktop.gtk.generated.NativeScreenInfoArray
import org.jetbrains.desktop.gtk.generated.NativeWindowParams
import org.jetbrains.desktop.gtk.generated.`application_run_on_event_loop_async$f`
import org.jetbrains.desktop.gtk.generated.desktop_gtk_h
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
    GL,
    GL_ES,
}

public data class WindowParams(
    val windowId: WindowId,
    val title: String,
    val size: LogicalSize,
    val minSize: LogicalSize?,
    val decorationMode: WindowDecorationMode,
    val renderingMode: RenderingMode,
) {
    init {
        check(size.width > 0 && size.height > 0) {
            "Invalid size (both width and height must be greater than zero)"
        }
        minSize?.let {
            check(it.width > 0 && it.height > 0) {
                "Invalid min size (both width and height must be greater than zero)"
            }
        }
    }

    internal fun toNative(arena: Arena): MemorySegment {
        val nativeWindowParams = NativeWindowParams.allocate(arena)
        NativeWindowParams.size(nativeWindowParams, LogicalSize(size.width, size.height).toNative(arena))
        NativeWindowParams.min_size(nativeWindowParams, LogicalSize(minSize?.width ?: 0, minSize?.height ?: 0).toNative(arena))
        NativeWindowParams.title(nativeWindowParams, title.toNativeUtf8(arena))
        NativeWindowParams.decoration_mode(nativeWindowParams, decorationMode.toNative(arena))
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

public class ApplicationConfig(
    public val eventHandler: EventHandler,
    public val queryDragAndDropTarget: (DragAndDropQueryData) -> DragAndDropQueryResponse,
    public val getDataTransferData: (DataSource, String) -> ByteArray?,
    public val windowCloseRequest: (WindowId) -> Boolean,
    public val applicationWantsToTerminate: () -> Boolean,
    public val getSurroundingText: (WindowId) -> TextInputSurroundingText?,
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

public class Application(public val appId: String) {
    private var applicationConfig: ApplicationConfig? = null

    private val activeArenas = mutableMapOf<Long, Arena>()
    private val runOnEventLoopAsyncQueue = ConcurrentLinkedQueue<() -> Unit>()
    private val runOnEventLoopAsyncFunc: MemorySegment = `application_run_on_event_loop_async$f`.allocate({
        ffiUpCall {
            runOnEventLoopAsyncQueue.poll().invoke()
        }
    }, Arena.global())

    init {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_gtk_h.application_init(appId.toNativeUtf8(arena))
            }
        }
    }

    private fun newPersistentArena(): Pair<Arena, Long> {
        val arena = Arena.ofConfined()
        val objId = (activeArenas.keys.maxOrNull() ?: 0) + 1
        activeArenas[objId] = arena
        return Pair(arena, objId)
    }

    // called from native
    private fun onObjDealloc(objId: Long) {
        activeArenas[objId]!!.close()
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

    // called from native
    private fun onGetDataTransferData(nativeDataSource: Int, nativeMimeType: MemorySegment): MemorySegment {
        val dataSource = DataSource.fromNative(nativeDataSource)
        val mimeType = readStringFromNativeU8Array(nativeMimeType)!!
        val result = applicationConfig?.getDataTransferData(dataSource, mimeType)
        val (arena, objId) = newPersistentArena()
        return result.toNativeTransferDataResponse(arena, objId)
    }

    // called from native
    private fun onQueryDragAndDropTarget(nativeQueryData: MemorySegment): MemorySegment {
        val queryData = DragAndDropQueryData.fromNative(nativeQueryData)
        val result = applicationConfig?.queryDragAndDropTarget(queryData) ?: DragAndDropQueryResponse(
            supportedActionsPerMime = emptyList(),
        )
        val (arena, objId) = newPersistentArena()
        return result.toNative(arena, objId)
    }

    // called from native
    private fun onRetrieveSurroundingText(windowId: Long): MemorySegment {
        val result = applicationConfig?.getSurroundingText(windowId)
        val (arena, objId) = newPersistentArena()
        return result.toNative(arena, objId)
    }

    // called from native
    private fun onWindowCloseRequest(windowId: Long): Boolean {
        return applicationConfig?.windowCloseRequest(windowId) ?: true
    }

    // called from native
    private fun onApplicationWantsToTerminate(): Boolean {
        return applicationConfig?.applicationWantsToTerminate() ?: true
    }

    public fun runEventLoop(applicationConfig: ApplicationConfig) {
        this.applicationConfig = applicationConfig
        ffiDownCall {
            desktop_gtk_h.application_run_event_loop(applicationCallbacks())
        }
    }

    public fun stopEventLoop() {
        ffiDownCall {
            desktop_gtk_h.application_stop_event_loop()
        }
    }

    public fun openURL(url: String, activationToken: String?): RequestId? {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                val nativeUrl = url.toNativeUtf8(arena)
                val nativeActivationToken = activationToken.toNativeUtf8(arena)
                RequestId.fromNativeResponse(desktop_gtk_h.application_open_url(nativeUrl, nativeActivationToken))
            }
        }
    }

    public fun openFileManager(path: String, activationToken: String?): RequestId? {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                val nativePath = path.toNativeUtf8(arena)
                val nativeActivationToken = activationToken.toNativeUtf8(arena)
                RequestId.fromNativeResponse(desktop_gtk_h.application_open_file_manager(nativePath, nativeActivationToken))
            }
        }
    }

    private fun applicationCallbacks(): MemorySegment {
        val arena = Arena.global()
        val callbacks = NativeApplicationCallbacks.allocate(arena)
        NativeApplicationCallbacks.obj_dealloc(callbacks, NativeFfiObjDealloc.allocate(::onObjDealloc, arena))
        NativeApplicationCallbacks.event_handler(callbacks, NativeEventHandler.allocate(::onEvent, arena))
        NativeApplicationCallbacks.query_drag_and_drop_target(
            callbacks,
            NativeFfiQueryDragAndDropTarget.allocate(::onQueryDragAndDropTarget, arena),
        )
        NativeApplicationCallbacks.get_data_transfer_data(
            callbacks,
            NativeFfiTransferDataGetter.allocate(::onGetDataTransferData, arena),
        )
        NativeApplicationCallbacks.retrieve_surrounding_text(
            callbacks,
            NativeFfiRetrieveSurroundingText.allocate(::onRetrieveSurroundingText, arena),
        )
        NativeApplicationCallbacks.window_close_request(
            callbacks,
            NativeFfiWindowCloseRequest.allocate(::onWindowCloseRequest, arena),
        )
        NativeApplicationCallbacks.application_wants_to_terminate(
            callbacks,
            NativeFfiApplicationWantsToTerminate.allocate(::onApplicationWantsToTerminate, arena),
        )
        return callbacks
    }

    public fun createWindow(params: WindowParams): Window {
        return Window(params)
    }

    public fun isEventLoopThread(): Boolean {
        return ffiDownCall {
            desktop_gtk_h.application_is_event_loop_thread()
        }
    }

    public fun runOnEventLoopAsync(f: () -> Unit) {
        ffiDownCall {
            runOnEventLoopAsyncQueue.add(f)
            desktop_gtk_h.application_run_on_event_loop_async(runOnEventLoopAsyncFunc)
        }
    }

    public fun allScreens(): AllScreens {
        return Arena.ofConfined().use { arena ->
            val screenInfoArray = ffiDownCall { desktop_gtk_h.screen_list(arena) }
            val screens = mutableListOf<Screen>()
            try {
                val ptr = NativeScreenInfoArray.ptr(screenInfoArray)
                val len = NativeScreenInfoArray.len(screenInfoArray)

                for (i in 0 until len) {
                    screens.add(Screen.fromNative(NativeScreenInfo.asSlice(ptr, i)))
                }
            } finally {
                ffiDownCall { desktop_gtk_h.screen_list_drop(screenInfoArray) }
            }
            AllScreens(screens)
        }
    }

    public data class GlProcFunc(
        val fPtr: Long,
        val ctxPtr: Long,
    )

    public fun getEglProcFunc(): GlProcFunc? {
        return Arena.ofConfined().use { arena ->
            val s = desktop_gtk_h.application_get_egl_proc_func(arena)
            val f = NativeGetGlProcFuncData.f(s)
            val ctx = NativeGetGlProcFuncData.ctx(s)
            if (ctx == MemorySegment.NULL) null else GlProcFunc(fPtr = f.address(), ctxPtr = ctx.address())
        }
    }

    public fun initializeGl(libPath: String): GlProcFunc? {
        return Arena.ofConfined().use { arena ->
            val nativePath = libPath.toNativeUtf8(arena)
            val s = desktop_gtk_h.application_init_gl(arena, nativePath)
            val f = NativeGetGlProcFuncData.f(s)
            val ctx = NativeGetGlProcFuncData.ctx(s)
            if (ctx == MemorySegment.NULL) null else GlProcFunc(fPtr = f.address(), ctxPtr = ctx.address())
        }
    }

    /** Will produce [Event.DataTransfer] event with a matching serial. */
    public fun clipboardPaste(serial: Int, supportedMimeTypes: List<String>) {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_gtk_h.application_clipboard_paste(serial, mimeTypesToNative(arena, supportedMimeTypes))
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
                desktop_gtk_h.application_clipboard_put(mimeTypesToNative(arena, mimeTypes))
            }
        }
    }

    public fun clipboardGetAvailableMimeTypes(): List<String> {
        val ffiCsvMimetypes = ffiDownCall { desktop_gtk_h.application_clipboard_get_available_mimetypes() }
        return try {
            splitCsv(ffiCsvMimetypes.getString(0))
        } finally {
            ffiDownCall { desktop_gtk_h.string_drop(ffiCsvMimetypes) }
        }
    }

    /** Will produce [Event.DataTransfer] event with a matching serial. */
    public fun primarySelectionPaste(serial: Int, supportedMimeTypes: List<String>) {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_gtk_h.application_primary_selection_paste(serial, mimeTypesToNative(arena, supportedMimeTypes))
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
                desktop_gtk_h.application_primary_selection_put(mimeTypesToNative(arena, mimeTypes))
            }
        }
    }

    public fun primarySelectionGetAvailableMimeTypes(): List<String> {
        val ffiCsvMimetypes = ffiDownCall { desktop_gtk_h.application_primary_selection_get_available_mimetypes() }
        return try {
            splitCsv(ffiCsvMimetypes.getString(0))
        } finally {
            ffiDownCall { desktop_gtk_h.string_drop(ffiCsvMimetypes) }
        }
    }

    public fun requestShowNotification(params: ShowNotificationParams): RequestId? {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val title = params.title.toNativeUtf8(arena)
                val body = params.body.toNativeUtf8(arena)
                val soundFilePath = params.soundFilePath.toNativeUtf8(arena)
                RequestId.fromNativeResponse(desktop_gtk_h.application_request_show_notification(title, body, soundFilePath))
            }
        }
    }

    public fun closeNotification(notificationId: UInt) {
        ffiDownCall {
            desktop_gtk_h.application_close_notification(notificationId.toInt())
        }
    }

    public fun requestRedrawDragIcon() {
        ffiDownCall {
            desktop_gtk_h.application_request_redraw_drag_icon()
        }
    }

    public fun stopDragAndDrop() {
        ffiDownCall {
            desktop_gtk_h.application_stop_drag_and_drop()
        }
    }

    public fun setPreferDarkTheme(value: Boolean) {
        ffiDownCall {
            desktop_gtk_h.application_set_prefer_dark_theme(value)
        }
    }
}
