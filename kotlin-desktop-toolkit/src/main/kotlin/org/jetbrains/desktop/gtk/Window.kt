package org.jetbrains.desktop.gtk

import org.jetbrains.desktop.gtk.generated.desktop_gtk_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public typealias WindowId = Long

public data class DragIconParams(
    public val renderingMode: RenderingMode,
    public val size: LogicalSize,
) {
    init {
        check(size.width > 0 && size.height > 0) {
            "Invalid size (both width and height must be greater than zero)"
        }
    }
}

public class StartDragAndDropParams(
    public val mimeTypes: List<String>,
    public val actions: Set<DragAndDropAction>,
    public val dragIconParams: DragIconParams?,
)

public class Window internal constructor(params: WindowParams) : AutoCloseable {
    public val windowId: WindowId = params.windowId

    init {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_gtk_h.window_create(params.toNative(arena))
            }
        }
    }

    override fun toString(): String {
        return "${javaClass.typeName}(windowId=$windowId)"
    }

    public fun requestRedraw() {
        ffiDownCall { desktop_gtk_h.window_request_redraw(windowId) }
    }

    public fun setTitle(title: String) {
        Arena.ofConfined().use { arena ->
            val nativeTitle = arena.allocateUtf8String(title)
            ffiDownCall { desktop_gtk_h.window_set_title(windowId, nativeTitle) }
        }
    }

    public fun setFullScreen() {
        ffiDownCall {
            desktop_gtk_h.window_set_fullscreen(windowId)
        }
    }

    public fun unsetFullScreen() {
        ffiDownCall {
            desktop_gtk_h.window_unset_fullscreen(windowId)
        }
    }

    public fun maximize() {
        ffiDownCall {
            desktop_gtk_h.window_maximize(windowId)
        }
    }

    public fun unmaximize() {
        ffiDownCall {
            desktop_gtk_h.window_unmaximize(windowId)
        }
    }

    public fun minimize() {
        ffiDownCall {
            desktop_gtk_h.window_minimize(windowId)
        }
    }

    public fun setMinSize(size: LogicalSize) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_gtk_h.window_set_min_size(windowId, size.toNative(arena))
            }
        }
    }

    public fun setPointerShape(shape: PointerShape) {
        ffiDownCall {
            desktop_gtk_h.window_set_pointer_shape(windowId, shape.toNative())
        }
    }

    /**
     * Start a drag&drop action with the data that can be interpreted in any of the provided MIME type formats.
     * Later, [ApplicationConfig.getDataTransferData] may be called, with [DataSource.DragAndDrop] argument,
     * to actually get the data with the specified MIME type.
     */
    public fun startDragAndDrop(params: StartDragAndDropParams) {
        Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_gtk_h.window_start_drag_and_drop(
                    windowId,
                    mimeTypesToNative(arena, params.mimeTypes),
                    params.actions.toNative(),
                    (params.dragIconParams?.renderingMode ?: RenderingMode.Auto).toNative(),
                    (params.dragIconParams?.size ?: LogicalSize(0, 0)).toNative(arena),
                )
            }
        }
    }

    /**
     * Will produce [Event.FileChooserResponse] event if there is clipboard content,
     * with the [Event.FileChooserResponse.requestId] field same as this function's return value.
     */
    public fun showOpenFileDialog(commonParams: FileDialog.CommonDialogParams, openParams: FileDialog.OpenDialogParams): RequestId? {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val requestIdVal = desktop_gtk_h.window_show_open_file_dialog(
                    windowId,
                    commonParams.toNative(arena),
                    openParams.toNative(arena),
                )
                RequestId.fromNativeResponse(requestIdVal)
            }
        }
    }

    /**
     * Will produce [Event.FileChooserResponse] event if there is clipboard content,
     * with the [Event.FileChooserResponse.requestId] field same as this function's return value.
     */
    public fun showSaveFileDialog(commonParams: FileDialog.CommonDialogParams, saveParams: FileDialog.SaveDialogParams): RequestId? {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                val requestIdVal = desktop_gtk_h.window_show_save_file_dialog(
                    windowId,
                    commonParams.toNative(arena),
                    saveParams.toNative(arena),
                )
                RequestId.fromNativeResponse(requestIdVal)
            }
        }
    }

    public fun activate(token: String?) {
        Arena.ofConfined().use { arena ->
            val nativeToken = token?.let { token -> arena.allocateUtf8String(token) } ?: run { MemorySegment.NULL }
            ffiDownCall {
                desktop_gtk_h.window_activate(windowId, nativeToken)
            }
        }
    }

    public fun textInputEnable(context: TextInputContext) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_gtk_h.window_text_input_enable(windowId, context.toNative(arena))
            }
        }
    }

    /** Should be called after any data in [TextInputContext] is changed, but only if [textInputEnable] was called beforehand. */
    public fun textInputUpdate(context: TextInputContext) {
        ffiDownCall {
            Arena.ofConfined().use { arena ->
                desktop_gtk_h.window_text_input_update(windowId, context.toNative(arena))
            }
        }
    }

    /** Disable Text Input support, if [textInputEnable] was called beforehand. */
    public fun textInputDisable() {
        ffiDownCall {
            desktop_gtk_h.window_text_input_disable(windowId)
        }
    }

    override fun close() {
        Logger.trace { "Window: closing window with id $windowId" }
        ffiDownCall {
            desktop_gtk_h.window_close(windowId)
        }
        Logger.trace { "Window: closed window with id $windowId" }
    }
}
