package org.jetbrains.desktop.linux

import kotlin.time.Duration
import kotlin.time.Duration.Companion.milliseconds

@JvmInline
public value class Timestamp(
    /** Count of milliseconds since some fixed but arbitrary moment in the past */
    private val value: Int,
) {
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

public data class SoftwareDrawData(val canvas: Long, val stride: Int) {
    internal companion object;
}

public class DataTransferContent(public val data: ByteArray, public val mimeTypes: List<String>) {
    internal companion object;
}

public data class DragAndDropQueryData(public val windowId: WindowId, public val point: LogicalPoint) {
    internal companion object;
}

public sealed class Event {
    internal companion object;

    public data class DataTransfer(
        val serial: Int,
        val data: DataTransferContent,
    ) : Event()

    public data class KeyDown(
        val keyCode: KeyCode,
        val characters: String?,
        val key: KeySym,
        val isRepeat: Boolean,
    ) : Event()

    public data class KeyUp(
        val keyCode: KeyCode,
        val characters: String?,
        val key: KeySym,
    ) : Event()

    public data class ModifiersChanged(
        val modifiers: KeyModifiers,
    ) : Event()

    public data class MouseMoved(
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseDragged(
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseEntered(
        val locationInWindow: LogicalPoint,
    ) : Event()

    public data class MouseExited(
        val locationInWindow: LogicalPoint,
    ) : Event()

    public data class MouseUp(
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class MouseDown(
        val button: MouseButton,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    public data class ScrollWheel(
        val scrollingDeltaX: LogicalPixels,
        val scrollingDeltaY: LogicalPixels,
        val locationInWindow: LogicalPoint,
        val timestamp: Timestamp,
    ) : Event()

    /** Indicates if the Text Input support is available.
     * Call [Application.textInputEnable] to enable it or [Application.textInputDisable] to disable it afterward.
     */
    public data class TextInputAvailability(val available: Boolean) : Event()

    /** The application must proceed by evaluating the changes in the following order:
     * 1. Replace the existing preedit string with the cursor.
     * 2. Delete the requested surrounding text.
     * 3. Insert the commit string with the cursor at its end.
     * 4. Calculate surrounding text to send.
     * 5. Insert the new preedit text in the cursor position.
     * 6. Place the cursor inside the preedit text.
     */
    public data class TextInput(
        val preeditStringData: TextInputPreeditStringData?,
        val commitStringData: TextInputCommitStringData?,
        val deleteSurroundingTextData: TextInputDeleteSurroundingTextData?,
    ) : Event()

    public data object WindowCloseRequest : Event()

    public data class WindowConfigure(
        val size: LogicalSize,
        val active: Boolean,
        val maximized: Boolean,
        val fullscreen: Boolean,
        val clientSideDecorations: Boolean,
        val capabilities: WindowCapabilities,
    ) : Event()

    public data class WindowFocusChange(
        val isKeyWindow: Boolean,
        val isMainWindow: Boolean,
    ) : Event()

    public data class WindowFullScreenToggle(
        val isFullScreen: Boolean,
    ) : Event()

    public data class WindowDraw(
        val softwareDrawData: SoftwareDrawData?,
        val size: PhysicalSize,
        val scale: Double,
    ) : Event()

    public data class WindowScaleChanged(
        val newScale: Double,
    ) : Event()

    public data class WindowScreenChange(
        val newScreenId: ScreenId,
    ) : Event()
}
