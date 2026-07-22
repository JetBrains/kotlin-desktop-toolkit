package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_TextCompositionSegment
import org.jetbrains.desktop.win32.generated.NativeBorrowedUtf8
import org.jetbrains.desktop.win32.generated.NativeCaretRectArgs
import org.jetbrains.desktop.win32.generated.NativeCaretRectCallback
import org.jetbrains.desktop.win32.generated.NativeDiscardMarkedTextCallback
import org.jetbrains.desktop.win32.generated.NativeInsertTextArgs
import org.jetbrains.desktop.win32.generated.NativeInsertTextCallback
import org.jetbrains.desktop.win32.generated.NativeSelectedRangeCallback
import org.jetbrains.desktop.win32.generated.NativeSetMarkedTextArgs
import org.jetbrains.desktop.win32.generated.NativeSetMarkedTextCallback
import org.jetbrains.desktop.win32.generated.NativeTextCompositionSegment
import org.jetbrains.desktop.win32.generated.NativeTextInputClient
import org.jetbrains.desktop.win32.generated.NativeTextRange
import org.jetbrains.desktop.win32.generated.NativeUnmarkTextCallback
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout

public interface TextInputClient {
    public fun selectedRange(): TextRange?
    public fun caretRect(range: TextRange): LogicalRect
    public fun insertText(text: String)
    public fun setMarkedText(text: String, selectedRange: TextRange?, segments: List<TextCompositionSegment>)
    public fun unmarkText()
    public fun discardMarkedText()

    public object Noop : TextInputClient {
        override fun selectedRange(): TextRange? = null
        override fun caretRect(range: TextRange): LogicalRect = LogicalRect(LogicalPoint.Zero, LogicalSize(0f, 0f))
        override fun insertText(text: String): Unit = Unit
        override fun setMarkedText(text: String, selectedRange: TextRange?, segments: List<TextCompositionSegment>): Unit = Unit
        override fun unmarkText(): Unit = Unit
        override fun discardMarkedText(): Unit = Unit
    }
}

public data class TextRange(
    public val location: Long,
    public val length: Long,
) {
    internal companion object {
        val notFound: TextRange = TextRange(-1L, 0L)
        fun fromNative(native: MemorySegment): TextRange = TextRange(NativeTextRange.location(native), NativeTextRange.length(native))
    }

    internal fun nullIfNotFound(): TextRange? = if (location == -1L) null else this

    internal fun toNative(result: MemorySegment) {
        NativeTextRange.location(result, location)
        NativeTextRange.length(result, length)
    }
}

public data class TextCompositionSegment(
    public val range: TextRange,
    public val attribute: TextCompositionAttribute,
)

public enum class TextCompositionAttribute {
    Input,
    TargetConverted,
    Converted,
    TargetNotConverted,
    InputError,
    FixedConverted,
    Unspecified,
    ;

    internal companion object {
        fun fromNative(value: Int): TextCompositionAttribute = when (value) {
            desktop_win32_h.NativeTextCompositionAttribute_Input() -> Input
            desktop_win32_h.NativeTextCompositionAttribute_TargetConverted() -> TargetConverted
            desktop_win32_h.NativeTextCompositionAttribute_Converted() -> Converted
            desktop_win32_h.NativeTextCompositionAttribute_TargetNotConverted() -> TargetNotConverted
            desktop_win32_h.NativeTextCompositionAttribute_InputError() -> InputError
            desktop_win32_h.NativeTextCompositionAttribute_FixedConverted() -> FixedConverted
            desktop_win32_h.NativeTextCompositionAttribute_Unspecified() -> Unspecified
            else -> error("Unexpected TextCompositionAttribute value: $value")
        }
    }
}

internal fun readBorrowedUtf8(native: MemorySegment): String {
    val pointer = NativeBorrowedUtf8.ptr(native)
    val length = NativeBorrowedUtf8.len(native)
    if (pointer == MemorySegment.NULL || length == 0L) return ""
    return pointer.asSlice(0, length).toArray(ValueLayout.JAVA_BYTE).decodeToString()
}

internal fun readSegments(native: MemorySegment): List<TextCompositionSegment> {
    val pointer = NativeBorrowedArray_TextCompositionSegment.ptr(native)
    val length = NativeBorrowedArray_TextCompositionSegment.len(native)
    if (pointer == MemorySegment.NULL || length == 0L) return emptyList()
    return List(Math.toIntExact(length)) { index ->
        val item = NativeTextCompositionSegment.asSlice(pointer, index.toLong())
        TextCompositionSegment(
            range = TextRange.fromNative(NativeTextCompositionSegment.range(item)),
            attribute = TextCompositionAttribute.fromNative(NativeTextCompositionSegment.attribute(item)),
        )
    }
}

internal class TextInputClientHolder : AutoCloseable {
    private val arena: Arena = Arena.ofShared()
    internal var textInputClient: TextInputClient = TextInputClient.Noop

    internal val native: MemorySegment = NativeTextInputClient.allocate(arena).also { table ->
        NativeTextInputClient.selected_range(table, NativeSelectedRangeCallback.allocate(this::selectedRangeCallback, arena))
        NativeTextInputClient.caret_rect(table, NativeCaretRectCallback.allocate(this::caretRectCallback, arena))
        NativeTextInputClient.insert_text(table, NativeInsertTextCallback.allocate(this::insertTextCallback, arena))
        NativeTextInputClient.set_marked_text(table, NativeSetMarkedTextCallback.allocate(this::setMarkedTextCallback, arena))
        NativeTextInputClient.unmark_text(table, NativeUnmarkTextCallback.allocate(this::unmarkTextCallback, arena))
        NativeTextInputClient.discard_marked_text(
            table,
            NativeDiscardMarkedTextCallback.allocate(this::discardMarkedTextCallback, arena),
        )
    }

    internal fun selectedRangeCallback(rangeOut: MemorySegment) = ffiUpCall {
        (textInputClient.selectedRange() ?: TextRange.notFound).toNative(rangeOut)
    }

    internal fun caretRectCallback(args: MemorySegment) = ffiUpCall {
        val range = TextRange.fromNative(NativeCaretRectArgs.range_in(args))
        textInputClient.caretRect(range).toNative(NativeCaretRectArgs.rect_out(args))
    }

    internal fun insertTextCallback(args: MemorySegment) = ffiUpCall {
        textInputClient.insertText(readBorrowedUtf8(NativeInsertTextArgs.text(args)))
    }

    internal fun setMarkedTextCallback(args: MemorySegment) = ffiUpCall {
        textInputClient.setMarkedText(
            text = readBorrowedUtf8(NativeSetMarkedTextArgs.text(args)),
            selectedRange = TextRange.fromNative(NativeSetMarkedTextArgs.selected_range(args)).nullIfNotFound(),
            segments = readSegments(NativeSetMarkedTextArgs.segments(args)),
        )
    }

    internal fun unmarkTextCallback() = ffiUpCall { textInputClient.unmarkText() }
    internal fun discardMarkedTextCallback() = ffiUpCall { textInputClient.discardMarkedText() }

    override fun close() {
        arena.close()
    }
}
