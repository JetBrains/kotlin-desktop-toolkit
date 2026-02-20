package org.jetbrains.desktop.gtk

import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds

@JvmInline
public value class Timestamp private constructor(
    /** Count of milliseconds since some fixed but arbitrary moment in the past */
    private val value: Long,
) {
    internal companion object {
        internal fun fromNative(value: Int): Timestamp {
            return Timestamp(value.toUInt().toLong())
        }
    }

    public fun toDuration(): Duration {
        return value.milliseconds
    }
}

public data class WindowCapabilities(
    /** `show_window_menu` is available. */
    public val windowMenu: Boolean,

    /** Window can be maximized and unmaximized. */
    public val maximize: Boolean,

    /** Window can be fullscreened and unfullscreened. */
    public val fullscreen: Boolean,

    /** Window can be minimized. */
    public val minimize: Boolean,
) {
    internal companion object;
}

public data class OpenGlDrawData(val framebuffer: Int) {
    internal companion object;
}

public class DataTransferContent(
    public val mimeType: String,
    public val data: ByteArray,
) {
    internal companion object;

    override fun toString(): String {
        return "DataTransferContent(mimeType=$mimeType, data len = ${data.size} bytes)"
    }
}

public data class DragAndDropQueryData(
    public val windowId: WindowId,
    public val locationInWindow: LogicalPoint,
) {
    internal companion object;
}

public enum class DragAndDropAction {
    Copy,
    Move,
    ;

    internal companion object;
}

public data class SupportedActionsForMime(
    public val supportedMimeType: String,
    public val supportedActions: Set<DragAndDropAction>,
    public val preferredAction: DragAndDropAction?,
) {
    internal companion object;
}

public data class DragAndDropQueryResponse(public val supportedActionsPerMime: List<SupportedActionsForMime>) {
    internal companion object;
}

public enum class WindowDecorationMode {
    /** The window should draw client side decorations. */
    Client,

    /** The server will draw window decorations. */
    Server,

    ;

    internal companion object;
}

@JvmInline
public value class RequestId private constructor(private val id: Int) {
    internal companion object {
        internal fun fromNativeResponse(value: Int): RequestId? {
            return if (value == 0) {
                null
            } else {
                RequestId(value)
            }
        }

        internal fun fromNativeField(value: Int) = RequestId(value)
    }
}

public sealed class Event {
    internal companion object;

    public data object ApplicationStarted : Event()

    public data class XdgDesktopSettingChange(val setting: XdgDesktopSetting) : Event()

    public data class DataTransferAvailable(
        val dataSource: DataSource,
        val mimeTypes: List<String>,
    ) : Event()

    /** Data received from clipboard or primary selection. For drag&drop, see [DropPerformed]. */
    public data class DataTransfer(
        val serial: Int,
        val content: DataTransferContent?,
    ) : Event()

    /** Data transfer for data from our application was canceled */
    public data class DataTransferCancelled(val dataSource: DataSource) : Event()

    public data class DisplayConfigurationChange(val screens: AllScreens) : Event()

    /** Drag&drop targeting our application left the specified window. */
    public data class DragAndDropLeave(val windowId: WindowId) : Event()

    /** Drag&drop targeting our window is finished, and we received data from it. */
    public data class DropPerformed(
        val windowId: WindowId,
        val content: DataTransferContent?,
        val action: DragAndDropAction?,
    ) : Event()

    /** Drag&drop that was initiated from our window has finished. */
    public data class DragAndDropFinished(
        val windowId: WindowId,
        val action: DragAndDropAction?,
    ) : Event()

    public data class DragIconDraw(
        val openGlDrawData: OpenGlDrawData,
        val size: PhysicalSize,
        val scale: Double,
    ) : Event()

    public data class FileChooserResponse(
        val requestId: RequestId,

        /** URL-encoded file paths */
        val files: List<String>,
    ) : Event()

    public data class KeyDown(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val characters: String?,
        val key: KeySym,
        val modifiers: Set<KeyModifiers>,
        val isRepeat: Boolean,
    ) : Event()

    public data class KeyUp(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val key: KeySym,
    ) : Event()

    public data class ModifiersChanged(
        val windowId: WindowId,
        val modifiers: Set<KeyModifiers>,
    ) : Event()

    public data class MouseMoved(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseEntered(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
    ) : Event()

    public data class MouseExited(val windowId: WindowId) : Event()

    public data class MouseUp(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseDown(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class NotificationClosed(
        val notificationId: UInt,

        /** Present only if notification was activated. By default, it has a value `"default"` */
        val action: String?,

        /** Present only if notification was activated, and the application has an associated `.desktop` file. */
        val activationToken: String?,
    ) : Event()

    public data class NotificationShown(
        val requestId: RequestId,

        /** Null if the request failed */
        val notificationId: UInt?,
    ) : Event()

    public data class ScrollWheel(
        val windowId: WindowId,
        val scrollingDeltaX: LogicalPixels,
        val scrollingDeltaY: LogicalPixels,
        val timestamp: Timestamp,
    ) : Event()

    /** The application must proceed by evaluating the changes in the following order:
     * 1. Replace the existing preedit string with the cursor.
     * 2. Delete the requested surrounding text.
     * 3. Insert the commit string with the cursor at its end.
     * 4. Calculate surrounding text to send.
     * 5. Insert the new preedit text in the cursor position.
     * 6. Place the cursor inside the preedit text.
     */
    public data class TextInput(
        val windowId: WindowId,
        val preeditStringData: TextInputPreeditStringData?,
        val commitStringData: TextInputCommitStringData?,
        val deleteSurroundingTextData: TextInputDeleteSurroundingTextData?,
    ) : Event()

    public data class ShouldRedraw(val windowId: WindowId) : Event()

    public data object ShouldRedrawDragIcon : Event()

    public data class WindowClosed(val windowId: WindowId) : Event()

    public data class WindowConfigure(
        val windowId: WindowId,
        val size: LogicalSize,
        val active: Boolean,
        val maximized: Boolean,
        val fullscreen: Boolean,
        val decorationMode: WindowDecorationMode,
        val capabilities: WindowCapabilities,
    ) : Event()

    public data class WindowKeyboardEnter(val windowId: WindowId) : Event()

    public data class WindowKeyboardLeave(val windowId: WindowId) : Event()

    public data class WindowDraw(
        val windowId: WindowId,
        val openGlDrawData: OpenGlDrawData,
        val size: PhysicalSize,
    ) : Event()

    public data class WindowScaleChanged(
        val windowId: WindowId,
        val newScale: Double,
    ) : Event()

    public data class WindowScreenChange(
        val windowId: WindowId,
        val newScreenId: ScreenId,
    ) : Event()
}
