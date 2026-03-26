package org.jetbrains.desktop.linux

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

// TODO: Internal constructor
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

@ConsistentCopyVisibility
public data class SoftwareDrawData internal constructor(
    val canvas: Long,
    val stride: Int,
) {
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

@ConsistentCopyVisibility
public data class ScrollData internal constructor(
    val delta: LogicalPixels,
    val wheelValue120: Int,
    val isInverted: Boolean,
    val isStop: Boolean,
) {
    internal companion object
}

public sealed class Event {
    internal companion object;

    public data object ApplicationStarted : Event()

    /** Return `true` from the event handler if the application should _not_ terminate. */
    public data object ApplicationWantsToTerminate : Event()

    public data object ApplicationWillTerminate : Event()

    @ConsistentCopyVisibility
    public data class XdgDesktopSettingChange internal constructor(val setting: XdgDesktopSetting) : Event()

    @ConsistentCopyVisibility
    public data class DataTransferAvailable internal constructor(
        val dataSource: DataSource,
        val mimeTypes: List<String>,
    ) : Event()

    /** Data received from clipboard or primary selection. For drag&drop, see [DropPerformed]. */
    @ConsistentCopyVisibility
    public data class DataTransfer internal constructor(
        val serial: Int,
        val content: DataTransferContent?,
    ) : Event()

    /** Data transfer for data from our application was canceled */
    @ConsistentCopyVisibility
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
    public data class DragIconDraw internal constructor(
        val softwareDrawData: SoftwareDrawData?,
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
    public data class ActivationTokenResponse internal constructor(
        val requestId: RequestId,
        val token: String,
    ) : Event()

    @ConsistentCopyVisibility
    public data class KeyDown internal constructor(
        val keyCode: KeyCode,
        val characters: String?,
        val key: KeySym,
        val isRepeat: Boolean,
    ) : Event()

    @ConsistentCopyVisibility
    public data class KeyUp internal constructor(
        val keyCode: KeyCode,
        val key: KeySym,
    ) : Event()

    @ConsistentCopyVisibility
    public data class ModifiersChanged internal constructor(val modifiers: Set<KeyModifiers>) : Event()

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
    public data class MouseExited internal constructor(
        val windowId: WindowId,
        val locationInWindow: LogicalPoint,
    ) : Event()

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
        @Deprecated("Use `horizontalScroll` instead")
        val scrollingDeltaX: LogicalPixels,
        @Deprecated("Use `verticalScroll` instead")
        val scrollingDeltaY: LogicalPixels,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
        val horizontalScroll: ScrollData,
        val verticalScroll: ScrollData,
    ) : Event()

    /** Indicates if the Text Input support is available.
     * Call [Application.textInputEnable] to enable it or [Application.textInputDisable] to disable it afterward.
     */
    @ConsistentCopyVisibility
    public data class TextInputAvailability internal constructor(
        val windowId: WindowId,
        val available: Boolean,
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
        val preeditStringData: TextInputPreeditStringData?,
        val commitStringData: TextInputCommitStringData?,
        val deleteSurroundingTextData: TextInputDeleteSurroundingTextData?,
    ) : Event()

    @ConsistentCopyVisibility
    public data class WindowCloseRequest internal constructor(val windowId: WindowId) : Event()

    @ConsistentCopyVisibility
    public data class WindowClosed internal constructor(val windowId: WindowId) : Event()

    // TODO: Internal constructor
    public data class WindowConfigure(
        val windowId: WindowId,
        val size: LogicalSize,
        val active: Boolean,
        val maximized: Boolean,
        val fullscreen: Boolean,
        val decorationMode: WindowDecorationMode,
        val capabilities: WindowCapabilities,
    ) : Event()

    @ConsistentCopyVisibility
    public data class WindowKeyboardEnter internal constructor(
        val windowId: WindowId,
        val keyCodes: List<KeyCode>,
        val keySyms: List<KeySym>,
    ) : Event()

    @ConsistentCopyVisibility
    public data class WindowKeyboardLeave internal constructor(val windowId: WindowId) : Event()

    @ConsistentCopyVisibility
    public data class WindowDraw internal constructor(
        val windowId: WindowId,
        val softwareDrawData: SoftwareDrawData?,
        val size: PhysicalSize,
        val scale: Double,
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
