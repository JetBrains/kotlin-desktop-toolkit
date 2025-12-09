package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeDragAndDropCallbacks
import org.jetbrains.desktop.macos.generated.NativeDragTargetEnteredCallback
import org.jetbrains.desktop.macos.generated.NativeDragTargetExitedCallback
import org.jetbrains.desktop.macos.generated.NativeDragTargetInfo
import org.jetbrains.desktop.macos.generated.NativeDragTargetPerformCallback
import org.jetbrains.desktop.macos.generated.NativeDragTargetUpdatedCallback
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
            val destinationWindowId = NativeDragTargetInfo.destination_window_id(segment)
            val locationInWindow = LogicalPoint.fromNative(NativeDragTargetInfo.location_in_window(segment))
            val allowedOperations = NativeDragTargetInfo.allowed_operations(segment)
            val sequenceNumber = NativeDragTargetInfo.sequence_number(segment)
            val pasteboardName = NativeDragTargetInfo.pasteboard_name(segment).getUtf8String(0)
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
public interface DragTargetCallbacks {
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
    private lateinit var dragTargetCallbacks: DragTargetCallbacks

    public fun init(callbacks: DragTargetCallbacks) {
        arena = Arena.ofConfined()
        dragTargetCallbacks = callbacks
        desktop_macos_h.set_drag_and_drop_callbacks(dragAndDropCallbacks())
    }

    private fun dragAndDropCallbacks(): MemorySegment {
        val callbacks = NativeDragAndDropCallbacks.allocate(arena)
        NativeDragAndDropCallbacks.drag_target_entered_callback(
            callbacks,
            NativeDragTargetEnteredCallback.allocate(::onDragEntered, arena),
        )
        NativeDragAndDropCallbacks.drag_target_updated_callback(
            callbacks,
            NativeDragTargetUpdatedCallback.allocate(::onDragUpdated, arena),
        )
        NativeDragAndDropCallbacks.drag_target_exited_callback(
            callbacks,
            NativeDragTargetExitedCallback.allocate(::onDragExited, arena),
        )
        NativeDragAndDropCallbacks.drag_target_perform_callback(
            callbacks,
            NativeDragTargetPerformCallback.allocate(::onDragPerformed, arena),
        )
        return callbacks
    }

    // called from native
    private fun onDragEntered(dragInfo: MemorySegment): Long {
        return ffiUpCall(defaultResult = DragOperation.NONE.value) {
            dragTargetCallbacks.onDragEntered(DragInfo.fromNative(dragInfo)).value
        }
    }

    // called from native
    private fun onDragUpdated(dragInfo: MemorySegment): Long {
        return ffiUpCall(defaultResult = DragOperation.NONE.value) {
            dragTargetCallbacks.onDragUpdated(DragInfo.fromNative(dragInfo)).value
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
            dragTargetCallbacks.onDragExited(dragInfo)
        }
    }

    // called from native
    private fun onDragPerformed(dragInfo: MemorySegment): Boolean {
        return ffiUpCall(defaultResult = false) {
            dragTargetCallbacks.onDragPerformed(DragInfo.fromNative(dragInfo))
        }
    }

    override fun close() {
        if (::arena.isInitialized) {
            desktop_macos_h.drop_drag_and_drop_callbacks()
            arena.close()
        }
    }
}
