package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeDragAndDropCallbacks
import org.jetbrains.desktop.macos.generated.NativeDragSourceOperationMaskCallback
import org.jetbrains.desktop.macos.generated.NativeDragSourceSessionEndedAt
import org.jetbrains.desktop.macos.generated.NativeDragSourceSessionMovedTo
import org.jetbrains.desktop.macos.generated.NativeDragSourceSessionWillBeginAt
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

/**
 * Bitset of drag operations.
 */
@JvmInline
public value class DragOperationsSet internal constructor(internal val value: Long) {
    public operator fun contains(operation: DragOperation): Boolean = (value and operation.value) != 0L

    public operator fun plus(operation: DragOperation): DragOperationsSet = DragOperationsSet(value or operation.value)

    public operator fun plus(other: DragOperationsSet): DragOperationsSet = DragOperationsSet(value or other.value)

    public companion object {
        public fun of(operation: DragOperation): DragOperationsSet = DragOperationsSet(operation.value)

        public val NONE: DragOperationsSet = DragOperationsSet(0L)
    }
}

public data class DragInfo(
    val destinationWindowId: WindowId,
    val locationInWindow: LogicalPoint,
    val allowedOperations: DragOperationsSet,
    val sequenceNumber: Long,
    val pasteboardName: String,
) {
    internal companion object {
        fun fromNative(segment: MemorySegment): DragInfo {
            val destinationWindowId = NativeDragTargetInfo.destination_window_id(segment)
            val locationInWindow = LogicalPoint.fromNative(NativeDragTargetInfo.location_in_window(segment))
            val allowedOperations = DragOperationsSet(NativeDragTargetInfo.allowed_operations(segment))
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
 * Dragging context constants.
 */
@JvmInline
public value class DraggingContext internal constructor(public val value: Long) {
    public companion object {
        public val OUTSIDE_APPLICATION: DraggingContext = DraggingContext(0L)
        public val WITHIN_APPLICATION: DraggingContext = DraggingContext(1L)
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
 * Callback interface for handling drag source operations.
 */
public interface DragSourceCallbacks {
    /**
     * Called to determine which drag operations are allowed for the drag session.
     * @param sourceWindowId The window that initiated the drag
     * @param sequenceNumber Unique identifier for this drag session
     * @param context The dragging context (within or outside application)
     * @return Bitset of allowed drag operations
     */
    public fun onDragSourceOperationMask(sourceWindowId: WindowId, sequenceNumber: Long, context: DraggingContext): DragOperationsSet =
        DragOperationsSet.NONE

    /**
     * Called when a drag session begins.
     * @param sourceWindowId The window that initiated the drag
     * @param sequenceNumber Unique identifier for this drag session
     * @param locationOnScreen The location where the drag began on the screen
     */
    public fun onDragSourceSessionWillBeginAt(sourceWindowId: WindowId, sequenceNumber: Long, locationOnScreen: LogicalPoint) {}

    /**
     * Called when the drag moves to a new location.
     * @param sourceWindowId The window that initiated the drag
     * @param sequenceNumber Unique identifier for this drag session
     * @param locationOnScreen The current location of the drag on the screen
     */
    public fun onDragSourceSessionMovedTo(sourceWindowId: WindowId, sequenceNumber: Long, locationOnScreen: LogicalPoint) {}

    /**
     * Called when the drag session ends.
     * @param sourceWindowId The window that initiated the drag
     * @param sequenceNumber Unique identifier for this drag session
     * @param locationOnScreen The location where the drag ended on the screen
     * @param dragOperation The final drag operation that was performed
     */
    public fun onDragSourceSessionEndedAt(
        sourceWindowId: WindowId,
        sequenceNumber: Long,
        locationOnScreen: LogicalPoint,
        dragOperation: DragOperation,
    ) {}
}

/**
 * Holder for drag and drop callbacks that manages native callback allocation.
 */
public object DragAndDropHandler : AutoCloseable {
    private lateinit var arena: Arena
    private lateinit var dragTargetCallbacks: DragTargetCallbacks
    private lateinit var dragSourceCallbacks: DragSourceCallbacks

    public fun init(targetCallbacks: DragTargetCallbacks, sourceCallbacks: DragSourceCallbacks) {
        arena = Arena.ofConfined()
        dragTargetCallbacks = targetCallbacks
        dragSourceCallbacks = sourceCallbacks
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
        NativeDragAndDropCallbacks.drag_source_operation_mask_callback(
            callbacks,
            NativeDragSourceOperationMaskCallback.allocate(::onDragSourceOperationMask, arena),
        )
        NativeDragAndDropCallbacks.drag_source_session_will_begin_at(
            callbacks,
            NativeDragSourceSessionWillBeginAt.allocate(::onDragSourceSessionWillBeginAt, arena),
        )
        NativeDragAndDropCallbacks.drag_source_session_moved_to(
            callbacks,
            NativeDragSourceSessionMovedTo.allocate(::onDragSourceSessionMovedTo, arena),
        )
        NativeDragAndDropCallbacks.drag_source_session_ended_at(
            callbacks,
            NativeDragSourceSessionEndedAt.allocate(::onDragSourceSessionEndedAt, arena),
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

    // called from native
    private fun onDragSourceOperationMask(sourceWindowId: Long, sequenceNumber: Long, context: Long): Long {
        return ffiUpCall(defaultResult = 0L) {
            dragSourceCallbacks.onDragSourceOperationMask(
                sourceWindowId,
                sequenceNumber,
                DraggingContext(context),
            ).value
        }
    }

    // called from native
    private fun onDragSourceSessionWillBeginAt(sourceWindowId: Long, sequenceNumber: Long, locationOnScreen: MemorySegment) {
        ffiUpCall {
            dragSourceCallbacks.onDragSourceSessionWillBeginAt(
                sourceWindowId,
                sequenceNumber,
                LogicalPoint.fromNative(locationOnScreen),
            )
        }
    }

    // called from native
    private fun onDragSourceSessionMovedTo(sourceWindowId: Long, sequenceNumber: Long, locationOnScreen: MemorySegment) {
        ffiUpCall {
            dragSourceCallbacks.onDragSourceSessionMovedTo(
                sourceWindowId,
                sequenceNumber,
                LogicalPoint.fromNative(locationOnScreen),
            )
        }
    }

    // called from native
    private fun onDragSourceSessionEndedAt(
        sourceWindowId: Long,
        sequenceNumber: Long,
        locationOnScreen: MemorySegment,
        dragOperation: Long,
    ) {
        ffiUpCall {
            dragSourceCallbacks.onDragSourceSessionEndedAt(
                sourceWindowId,
                sequenceNumber,
                LogicalPoint.fromNative(locationOnScreen),
                DragOperation(dragOperation),
            )
        }
    }

    override fun close() {
        if (::arena.isInitialized) {
            desktop_macos_h.drop_drag_and_drop_callbacks()
            arena.close()
        }
    }
}
