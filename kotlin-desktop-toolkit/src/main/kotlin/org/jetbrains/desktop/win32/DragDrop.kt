package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeDragSourceCallbacks
import org.jetbrains.desktop.win32.generated.NativeDropTargetCallbacks
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class DragDropManager(private val window: Window) : AutoCloseable {
    private val arena = Arena.ofConfined()

    private var dropTargetCallbacks: DropTargetCallbacks? = null

    public fun registerDropTarget(dropTarget: DropTarget) {
        check(dropTargetCallbacks == null) {
            "Drop target already registered. Please revoke the previous drop target before registering a new one."
        }
        dropTargetCallbacks = DropTargetCallbacks(arena, dropTarget).also { callbacks ->
            window.withPointer { windowPtr ->
                ffiDownCall {
                    desktop_win32_h.drag_drop_register_target(windowPtr, callbacks.toNative())
                }
            }
        }
    }

    public fun doDragDrop(dataFormat: DataFormat, data: ByteArray, dragSource: DragSource) {
        DragSourceCallbacks(arena, dragSource).use { callbacks ->
            ffiDownCall {
                desktop_win32_h.drag_drop_start(dataFormat.id, data.toNative(arena), callbacks.toNative())
            }
        }
    }

    public fun revokeDropTarget() {
        window.withPointer { windowPtr ->
            ffiDownCall {
                desktop_win32_h.drag_drop_revoke_target(windowPtr)
            }
        }
        dropTargetCallbacks?.close()
        dropTargetCallbacks = null
    }

    override fun close() {
        dropTargetCallbacks?.close()
        arena.close()
    }
}

public interface DropTarget {
    /** NOTE: The implementer must release the DataObject when it is done using it! */
    public fun onDragEnter(
        dataObject: DataObject,
        modifiers: DragDropModifiers,
        point: PhysicalPoint,
        effect: DragDropEffects,
    ): DragDropEffects

    public fun onDragOver(modifiers: DragDropModifiers, point: PhysicalPoint, effect: DragDropEffects): DragDropEffects

    public fun onDragLeave()

    /** NOTE: The implementer must release the DataObject when it is done using it! */
    public fun onDrop(dataObject: DataObject, modifiers: DragDropModifiers, point: PhysicalPoint, effect: DragDropEffects): DragDropEffects
}

public interface DragSource {
    public fun onQueryContinueDrag(escapePressed: Boolean, modifiers: DragDropModifiers): DragDropContinueResult
}

@JvmInline
public value class DragDropEffects internal constructor(internal val value: Int) {
    public infix fun and(other: DragDropEffect): DragDropEffects {
        return DragDropEffects(this.value and other.value)
    }
}

@JvmInline
public value class DragDropEffect private constructor(internal val value: Int) {
    public companion object {
        public val None: DragDropEffect = DragDropEffect(0)
        public val Copy: DragDropEffect = DragDropEffect(1)
        public val Move: DragDropEffect = DragDropEffect(2)
        public val Link: DragDropEffect = DragDropEffect(4)
        public val Scroll: DragDropEffect = DragDropEffect(2_147_483_648.toInt())
    }
}

public enum class DragDropContinueResult {
    Continue,
    Cancel,
    Drop, ;

    internal fun toNative(): Int = when (this) {
        Continue -> desktop_win32_h.NativeDragDropContinueResult_Continue()
        Cancel -> desktop_win32_h.NativeDragDropContinueResult_Cancel()
        Drop -> desktop_win32_h.NativeDragDropContinueResult_Drop()
    }
}

private class DropTargetCallbacks(
    arena: Arena,
    private val target: DropTarget,
) : AutoCloseable {
    private val callbacks: MemorySegment = NativeDropTargetCallbacks.allocate(arena)

    init {
        NativeDropTargetCallbacks.drag_enter_handler(
            callbacks,
            NativeDropTargetCallbacks.drag_enter_handler.allocate(::dragEnter, arena),
        )

        NativeDropTargetCallbacks.drag_over_handler(
            callbacks,
            NativeDropTargetCallbacks.drag_over_handler.allocate(::dragOver, arena),
        )

        NativeDropTargetCallbacks.drag_leave_handler(
            callbacks,
            NativeDropTargetCallbacks.drag_leave_handler.allocate(::dragLeave, arena),
        )

        NativeDropTargetCallbacks.drop_handler(
            callbacks,
            NativeDropTargetCallbacks.drop_handler.allocate(::drop, arena),
        )
    }

    fun dragEnter(dataObj: MemorySegment, keyState: Int, point: MemorySegment, effect: Int): Int {
        val result =
            target.onDragEnter(DataObject(dataObj), DragDropModifiers(keyState), PhysicalPoint.fromNative(point), DragDropEffects(effect))
        return result.value
    }

    fun dragOver(keyState: Int, point: MemorySegment, effect: Int): Int {
        val result = target.onDragOver(DragDropModifiers(keyState), PhysicalPoint.fromNative(point), DragDropEffects(effect))
        return result.value
    }

    fun dragLeave() = target.onDragLeave()

    fun drop(dataObj: MemorySegment, keyState: Int, point: MemorySegment, effect: Int): Int {
        val result =
            target.onDrop(DataObject(dataObj), DragDropModifiers(keyState), PhysicalPoint.fromNative(point), DragDropEffects(effect))
        return result.value
    }

    fun toNative(): MemorySegment = callbacks

    override fun close() {
        // TODO: drop native callbacks
    }
}

private class DragSourceCallbacks(
    arena: Arena,
    private val source: DragSource,
) : AutoCloseable {
    private val callbacks: MemorySegment = NativeDragSourceCallbacks.allocate(arena)

    init {
        NativeDragSourceCallbacks.query_continue_drag_handler(
            callbacks,
            NativeDragSourceCallbacks.query_continue_drag_handler.allocate(::queryContinueDrag, arena),
        )
    }

    fun queryContinueDrag(escapePressed: Boolean, keyState: Int): Int {
        val result = source.onQueryContinueDrag(escapePressed, DragDropModifiers(keyState))
        return result.toNative()
    }

    fun toNative(): MemorySegment = callbacks

    override fun close() {
        // TODO: drop native callbacks
    }
}

@JvmInline
public value class DragDropModifier private constructor(internal val value: Int) {
    // See https://learn.microsoft.com/en-us/windows/win32/api/oleidl/nf-oleidl-idroptarget-dragenter
    public companion object {
        public val None: DragDropModifier = DragDropModifier(0)
        public val Control: DragDropModifier = DragDropModifier(8)
        public val Shift: DragDropModifier = DragDropModifier(4)
        public val Alt: DragDropModifier = DragDropModifier(32)
        public val LeftButton: DragDropModifier = DragDropModifier(1)
        public val MiddleButton: DragDropModifier = DragDropModifier(16)
        public val RightButton: DragDropModifier = DragDropModifier(2)
    }
}

@JvmInline
public value class DragDropModifiers internal constructor(private val value: Int) {
    public fun hasFlag(modifier: DragDropModifier): Boolean {
        return (this.value and modifier.value) == modifier.value
    }
}
