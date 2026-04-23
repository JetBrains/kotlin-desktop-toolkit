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

@ConsistentCopyVisibility
public data class OpenGlDrawData internal constructor(
    val framebuffer: Int,
    val isEs: Boolean,
) {
    internal companion object;
}

public class DataTransferContent internal constructor(
    public val mimeType: String,
    public val data: ByteArray,
) {
    internal companion object;

    override fun toString(): String {
        return "DataTransferContent(mimeType=$mimeType, data len = ${data.size} bytes)"
    }
}

@ConsistentCopyVisibility
public data class DragAndDropQueryData internal constructor(
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

public sealed class WindowDecorationMode {
    public data class CustomTitlebar(val height: Int) : WindowDecorationMode()

    /** The server will draw window decorations. */
    public object Server : WindowDecorationMode() {
        override fun toString(): String {
            return "Server"
        }
    }

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

    public class ApplicationStarted internal constructor() : Event()

    @ConsistentCopyVisibility
    public data class DesktopSettingChange internal constructor(val setting: DesktopSetting) : Event()

    @ConsistentCopyVisibility
    public data class DataTransferAvailable internal constructor(
        val dataSource: DataSource,
        val mimeTypes: List<String>,
    ) : Event()

    @ConsistentCopyVisibility
    /** Data received from clipboard or primary selection. For drag&drop, see [DropPerformed]. */
    public data class DataTransfer internal constructor(
        val serial: Int,
        val content: DataTransferContent?,
    ) : Event()

    @ConsistentCopyVisibility
    /** Data transfer for data from our application was canceled */
    public data class DataTransferCancelled internal constructor(val dataSource: DataSource) : Event()

    @ConsistentCopyVisibility
    public data class DisplayConfigurationChange internal constructor(val screens: AllScreens) : Event()

    @ConsistentCopyVisibility
    /** Drag&drop targeting our application left the specified window. */
    public data class DragAndDropLeave internal constructor(val windowId: WindowId) : Event()

    @ConsistentCopyVisibility
    /** Drag&drop targeting our window is finished, and we received data from it. */
    public data class DropPerformed internal constructor(
        val windowId: WindowId,
        val content: DataTransferContent?,
        val action: DragAndDropAction?,
        val locationInWindow: LogicalPoint,
    ) : Event()

    @ConsistentCopyVisibility
    /** Drag&drop that was initiated from our window has finished. */
    public data class DragAndDropFinished internal constructor(
        val windowId: WindowId,
        val action: DragAndDropAction?,
    ) : Event()

    @ConsistentCopyVisibility
    /** The UI feedback for drag&drop that was initiated from our window has finished (e.g., the animation has finished). */
    public data class DragAndDropFeedbackFinished internal constructor(val windowId: WindowId) : Event()

    public class DragIconFrameTick internal constructor() : Event()

    @ConsistentCopyVisibility
    public data class DragIconDraw internal constructor(
        val openGlDrawData: OpenGlDrawData,
        val size: PhysicalSize,
        val scale: Double,
    ) : Event()

    @ConsistentCopyVisibility
    public data class FileChooserResponse internal constructor(
        val requestId: RequestId,

        /** URL-encoded file paths */
        val files: List<String>,
    ) : Event()

    @ConsistentCopyVisibility
    public data class KeyDown internal constructor(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val characters: String?,
        val key: KeySym,
        val modifiers: Set<KeyModifiers>,
    ) : Event()

    @ConsistentCopyVisibility
    public data class KeyUp internal constructor(
        val windowId: WindowId,
        val keyCode: KeyCode,
        val key: KeySym,
    ) : Event()

    @ConsistentCopyVisibility
    public data class ModifiersChanged internal constructor(
        val windowId: WindowId,
        val modifiers: Set<KeyModifiers>,
    ) : Event()

    @ConsistentCopyVisibility
    public data class MouseMoved internal constructor(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    @ConsistentCopyVisibility
    public data class MouseEntered internal constructor(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
    ) : Event()

    @ConsistentCopyVisibility
    public data class MouseExited internal constructor(val windowId: WindowId) : Event()

    @ConsistentCopyVisibility
    public data class MouseUp internal constructor(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    @ConsistentCopyVisibility
    public data class MouseDown internal constructor(
        val windowId: WindowId,
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    @ConsistentCopyVisibility
    public data class NotificationClosed internal constructor(
        val notificationId: UInt,

        /** Present only if notification was activated. By default, it has a value `"default"` */
        val action: String?,

        /** Present only if notification was activated, and the application has an associated `.desktop` file. */
        val activationToken: String?,
    ) : Event()

    @ConsistentCopyVisibility
    public data class NotificationShown internal constructor(
        val requestId: RequestId,

        /** Null if the request failed */
        val notificationId: UInt?,
    ) : Event()

    @ConsistentCopyVisibility
    public data class ScrollWheel internal constructor(
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
    @ConsistentCopyVisibility
    public data class TextInput internal constructor(
        val windowId: WindowId,
        val preeditStringData: TextInputPreeditStringData?,
        val commitStringData: TextInputCommitStringData?,
        val deleteSurroundingTextData: TextInputDeleteSurroundingTextData?,
    ) : Event()

    @ConsistentCopyVisibility
    public data class WindowFrameTick internal constructor(
        val windowId: WindowId,
        val frameTimeMicroseconds: Long,
    ) : Event()

    @ConsistentCopyVisibility
    public data class WindowClosed internal constructor(val windowId: WindowId) : Event()

    @ConsistentCopyVisibility
    public data class WindowConfigure internal constructor(
        val windowId: WindowId,
        val size: LogicalSize,
        val active: Boolean,
        val maximized: Boolean,
        val fullscreen: Boolean,
        val decorationMode: WindowDecorationMode,
        val insetStart: LogicalSize,
        val insetEnd: LogicalSize,
    ) : Event()

    @ConsistentCopyVisibility
    public data class WindowKeyboardEnter internal constructor(val windowId: WindowId) : Event()

    @ConsistentCopyVisibility
    public data class WindowKeyboardLeave internal constructor(val windowId: WindowId) : Event()

    @ConsistentCopyVisibility
    public data class WindowDraw internal constructor(
        val windowId: WindowId,
        val openGlDrawData: OpenGlDrawData,
        val size: PhysicalSize,
    ) : Event()

    @ConsistentCopyVisibility
    public data class WindowScaleChanged internal constructor(
        val windowId: WindowId,
        val newScale: Double,
    ) : Event()

    @ConsistentCopyVisibility
    public data class WindowScreenChange internal constructor(
        val windowId: WindowId,
        val newScreenId: ScreenId,
    ) : Event()
}
