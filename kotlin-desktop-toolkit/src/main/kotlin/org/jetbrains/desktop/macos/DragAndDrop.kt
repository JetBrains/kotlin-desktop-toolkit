package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeDragAndDropCallbacks
import org.jetbrains.desktop.macos.generated.NativeDragEnteredCallback
import org.jetbrains.desktop.macos.generated.NativeDragExitedCallback
import org.jetbrains.desktop.macos.generated.NativeDragInfo
import org.jetbrains.desktop.macos.generated.NativeDragPerformCallback
import org.jetbrains.desktop.macos.generated.NativeDragUpdatedCallback
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

/**
 * Drag operation constants that match the native DragOperation type.
 */
@JvmInline
public value class DragOperation internal constructor(internal val value: Long) {
    public companion object {
        public val NONE: DragOperation = DragOperation(0L)
        public val COPY: DragOperation = DragOperation(1L)
        public val LINK: DragOperation = DragOperation(2L)
        public val GENERIC: DragOperation = DragOperation(4L)
        public val PRIVATE: DragOperation = DragOperation(8L)
        public val MOVE: DragOperation = DragOperation(16L)
        public val DELETE: DragOperation = DragOperation(32L)
    }
}

public data class DragInfo(
    val destinationWindowId: WindowId,
    val locationInWindow: LogicalPoint,
    val allowedOperations: Long,
    val sequenceNumber: Long,
    val pasteboardName: String,
) {
    internal companion object {
        fun fromNative(segment: MemorySegment): DragInfo {
            val destinationWindowId = NativeDragInfo.destination_window_id(segment)
            val locationInWindow = LogicalPoint.fromNative(NativeDragInfo.location_in_window(segment))
            val allowedOperations = NativeDragInfo.allowed_operations(segment)
            val sequenceNumber = NativeDragInfo.sequence_number(segment)
            val pasteboardName = NativeDragInfo.pasteboard_name(segment).getUtf8String(0)
            return DragInfo(
                destinationWindowId,
                locationInWindow,
                allowedOperations,
                sequenceNumber,
                pasteboardName,
            )
        }
    }
}

/**
 * Callback interface for handling drag and drop operations.
 */
public interface DragAndDropCallbacks {
    /**
     * Called when a drag operation enters the window.
     * @param info Information about the drag operation
     * @return The drag operation to accept, or DragOperation.NONE to reject
     */
    public fun onDragEntered(info: DragInfo): DragOperation = DragOperation.NONE

    /**
     * Called when a drag operation is updated (mouse moved) within the window.
     * @param info Information about the drag operation
     * @return The drag operation to accept, or DragOperation.NONE to reject
     */
    public fun onDragUpdated(info: DragInfo): DragOperation = DragOperation.NONE

    /**
     * Called when a drag operation exits the window.
     * @param info Information about the drag operation
     */
    public fun onDragExited(info: DragInfo?) {}

    /**
     * Called when a drag operation is performed (dropped) in the window.
     * @param info Information about the drag operation
     * @return true if the drop was accepted and handled, false otherwise
     */
    public fun onDragPerformed(info: DragInfo): Boolean = false
}

/**
 * Holder for drag and drop callbacks that manages native callback allocation.
 */
public object DragAndDropHandler : AutoCloseable {
    private lateinit var arena: Arena
    private lateinit var dragAndDropCallbacks: DragAndDropCallbacks

    public fun init(callbacks: DragAndDropCallbacks) {
        arena = Arena.ofConfined()
        dragAndDropCallbacks = callbacks
        desktop_macos_h.set_drag_and_drop_callbacks(dragAndDropCallbacks())
    }

    private fun dragAndDropCallbacks(): MemorySegment {
        val callbacks = NativeDragAndDropCallbacks.allocate(arena)
        NativeDragAndDropCallbacks.drag_entered_callback(
            callbacks,
            NativeDragEnteredCallback.allocate(::onDragEntered, arena),
        )
        NativeDragAndDropCallbacks.drag_updated_callback(
            callbacks,
            NativeDragUpdatedCallback.allocate(::onDragUpdated, arena),
        )
        NativeDragAndDropCallbacks.drag_exited_callback(
            callbacks,
            NativeDragExitedCallback.allocate(::onDragExited, arena),
        )
        NativeDragAndDropCallbacks.drag_perform_callback(
            callbacks,
            NativeDragPerformCallback.allocate(::onDragPerformed, arena),
        )
        return callbacks
    }

    // called from native
    private fun onDragEntered(dragInfo: MemorySegment): Long {
        return ffiUpCall(defaultResult = DragOperation.NONE.value) {
            dragAndDropCallbacks.onDragEntered(DragInfo.fromNative(dragInfo)).value
        }
    }

    // called from native
    private fun onDragUpdated(dragInfo: MemorySegment): Long {
        return ffiUpCall(defaultResult = DragOperation.NONE.value) {
            dragAndDropCallbacks.onDragUpdated(DragInfo.fromNative(dragInfo)).value
        }
    }

    // called from native
    private fun onDragExited(dragInfo: MemorySegment) {
        ffiUpCall {
            val dragInfo = if (dragInfo == MemorySegment.NULL) {
                null
            } else {
                DragInfo.fromNative(dragInfo)
            }
            dragAndDropCallbacks.onDragExited(dragInfo)
        }
    }

    // called from native
    private fun onDragPerformed(dragInfo: MemorySegment): Boolean {
        return ffiUpCall(defaultResult = false) {
            dragAndDropCallbacks.onDragPerformed(DragInfo.fromNative(dragInfo))
        }
    }

    override fun close() {
        if (::arena.isInitialized) {
            desktop_macos_h.drop_drag_and_drop_callbacks()
            arena.close()
        }
    }
}
