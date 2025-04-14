package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeAttributedStringForRangeCallback
import org.jetbrains.desktop.macos.generated.NativeAttributedStringForRangeResult
import org.jetbrains.desktop.macos.generated.NativeDoCommandCallback
import org.jetbrains.desktop.macos.generated.NativeFirstRectForCharacterRangeArgs
import org.jetbrains.desktop.macos.generated.NativeFirstRectForCharacterRangeCallback
import org.jetbrains.desktop.macos.generated.NativeFreeAttributedStringCallback
import org.jetbrains.desktop.macos.generated.NativeHasMarkedTextCallback
import org.jetbrains.desktop.macos.generated.NativeInsertTextArgs
import org.jetbrains.desktop.macos.generated.NativeInsertTextCallback
import org.jetbrains.desktop.macos.generated.NativeMarkedRangeCallback
import org.jetbrains.desktop.macos.generated.NativeSelectedRangeCallback
import org.jetbrains.desktop.macos.generated.NativeSetMarkedTextArgs
import org.jetbrains.desktop.macos.generated.NativeSetMarkedTextCallback
import org.jetbrains.desktop.macos.generated.NativeTextInputClient
import org.jetbrains.desktop.macos.generated.NativeTextRange
import org.jetbrains.desktop.macos.generated.NativeUnmarkTextCallback
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class TextInputContext(internal val window: Window) {
    public fun handleCurrentEvent(): EventHandlerResult {
        val wasHandled = ffiDownCall {
            desktop_macos_h.text_input_context_handle_current_event(window.pointer)
        }
        return if (wasHandled) {
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }

    public fun discardMarkedText() {
        ffiDownCall {
            desktop_macos_h.text_input_context_discard_marked_text(window.pointer)
        }
    }

    public fun invalidateCharacterCoordinates() {
        ffiDownCall {
            desktop_macos_h.text_input_context_invalidate_character_coordinates(window.pointer)
        }
    }

    public companion object {
        public fun beep() {
            ffiDownCall {
                desktop_macos_h.text_input_context_beep()
            }
        }

        internal val notFoundOffset by lazy {
            ffiDownCall {
                desktop_macos_h.text_input_context_not_found_offset()
            }
        }
    }
}

/**
 * If you need some inspiration for implementation, check out those links:
 * https://developer.apple.com/library/archive/samplecode/TextInputView/Introduction/Intro.html
 * https://github.com/jessegrosjean/NSTextInputClient
 */
public interface TextInputClient {
    public fun hasMarkedText(): Boolean

    /**
     * The returned range measures from the start of the receiver’s text storage.
     */
    public fun markedRange(): TextRange?

    /**
     * The returned range measures from the start of the receiver’s text storage, that is, from 0 to the document length.
     */
    public fun selectedRange(): TextRange?
    public fun insertText(text: String, replacementRange: TextRange?)
    public fun doCommand(command: String): Boolean

    /**
     * The receiver removes any marking from pending input text and disposes of the marked text as it wishes.
     * The text view should accept the marked text as if it had been inserted normally.
     * If there is no marked text, the invocation of this method has no effect.
     */
    public fun unmarkText()

    /**
     * text: The string to insert.
     * selectedRange: The range to set as the selection, computed from the beginning of the inserted string.
     * replacementRange: The range to replace, computed from the beginning of the marked text.
     *
     * If there is no marked text, the current selection is replaced. If there is no selection, the string is inserted at the insertion point.
     */
    public fun setMarkedText(text: String, selectedRange: TextRange?, replacementRange: TextRange?)

    public data class StringAndRange(val text: String?, val actualRange: TextRange?)
    public fun attributedStringForRange(range: TextRange): StringAndRange
    public data class RectAndRange(
        val rect: LogicalRect,
        val actualRange: TextRange?,
    )
    public fun firstRectForCharacterRange(range: TextRange): RectAndRange

    /**
     * The character index, measured from the start of the receiver’s text storage,
     *  of the character containing the given point.
     *  Returns NSNotFound if the cursor is not within a character’s bounding rectangle.
     */
    public fun characterIndexForPoint(point: LogicalPoint): Long?

    public object Noop : TextInputClient {
        override fun hasMarkedText(): Boolean {
            return false
        }

        override fun markedRange(): TextRange? {
            return null
        }

        override fun selectedRange(): TextRange? {
            return null
        }

        override fun insertText(text: String, replacementRange: TextRange?) {
        }

        override fun doCommand(command: String): Boolean {
            return false
        }

        override fun unmarkText() {
        }

        override fun setMarkedText(text: String, selectedRange: TextRange?, replacementRange: TextRange?) {
        }

        override fun attributedStringForRange(range: TextRange): StringAndRange {
            return StringAndRange("", range)
        }

        override fun firstRectForCharacterRange(range: TextRange): RectAndRange {
            return RectAndRange(LogicalRect.Zero, range)
        }

        override fun characterIndexForPoint(point: LogicalPoint): Long? {
            return null
        }
    }
}

/**
 * Range in text measure in UTF-16 characters.
 */
public data class TextRange(
    public val location: Long,
    public val length: Long,
) {
    public companion object {
        internal val notFound: TextRange = TextRange(
            location = TextInputContext.notFoundOffset,
            length = 0,
        )

        internal fun fromNative(native: MemorySegment): TextRange {
            return TextRange(NativeTextRange.location(native), NativeTextRange.length(native))
        }
    }

    internal fun nullIfNotFound(): TextRange? {
        return if (this == notFound) {
            null
        } else {
            this
        }
    }

    internal fun toNative(result: MemorySegment) {
        NativeTextRange.location(result, location)
        NativeTextRange.length(result, length)
    }
}

internal data class TextInputClientHolder(var textInputClient: TextInputClient = TextInputClient.Noop) : AutoCloseable {
    private val arena = Arena.ofShared()

    // called from native code
    private fun hasMarkedTextCallback(): Boolean {
        return ffiUpCall(defaultResult = false) {
            textInputClient.hasMarkedText()
        }
    }

    // called from native code
    private fun markedRangeCallback(rangeOut: MemorySegment) {
        ffiUpCall {
            val range = textInputClient.markedRange() ?: TextRange.notFound
            range.toNative(rangeOut)
        }
    }

    // called from native code
    private fun selectedRangeCallback(rangeOut: MemorySegment) {
        ffiUpCall {
            val range = textInputClient.selectedRange() ?: TextRange.notFound
            range.toNative(rangeOut)
        }
    }

    // called from native code
    private fun insertTextCallback(s: MemorySegment) {
        ffiUpCall {
            val text = NativeInsertTextArgs.text(s).getUtf8String(0)
            val replacementRange = TextRange.fromNative(NativeInsertTextArgs.replacement_range(s)).nullIfNotFound()
            textInputClient.insertText(text = text, replacementRange = replacementRange)
        }
    }

    // called from native code
    private fun setMarkedTextCallback(args: MemorySegment) {
        ffiUpCall {
            val text = NativeSetMarkedTextArgs.text(args).getUtf8String(0)
            val selectedRange = TextRange.fromNative(NativeSetMarkedTextArgs.selected_range(args)).nullIfNotFound()
            val replacementRange = TextRange.fromNative(NativeSetMarkedTextArgs.replacement_range(args)).nullIfNotFound()
            textInputClient.setMarkedText(text, selectedRange = selectedRange, replacementRange = replacementRange)
        }
    }

    // called from native code
    private fun unmarkTextCallback() {
        ffiUpCall {
            textInputClient.unmarkText()
        }
    }

    var arenaToRelease: Arena? = null

    // called from native code
    private fun attributedStringForRangeCallback(rangeIn: MemorySegment): MemorySegment {
        return ffiUpCall(defaultResult = MemorySegment.NULL) {
            val localArena = Arena.ofShared()
            val result = NativeAttributedStringForRangeResult.allocate(localArena)
            try {
                val rangeIn = TextRange.fromNative(rangeIn)
                assert(rangeIn.nullIfNotFound() != null)
                val stringAndRange = textInputClient.attributedStringForRange(rangeIn)

                (stringAndRange.actualRange ?: TextRange.notFound).toNative(NativeAttributedStringForRangeResult.actual_range(result))
                if (stringAndRange.text != null) {
                    NativeAttributedStringForRangeResult.string(result, localArena.allocateUtf8String(stringAndRange.text))
                } else {
                    NativeAttributedStringForRangeResult.string(result, MemorySegment.NULL)
                }
            } finally {
                assert(arenaToRelease == null)
                arenaToRelease = localArena
            }
            result
        }
    }

    // called from native code
    private fun freeAttributedStringForRangeCallback() {
        ffiUpCall {
            arenaToRelease!!.close()
            arenaToRelease = null
        }
    }

    // called from native code
    private fun firstRectForCharacterRangeCallback(args: MemorySegment) {
        ffiUpCall {
            val range = TextRange.fromNative(NativeFirstRectForCharacterRangeArgs.range_in(args))
            assert(range.nullIfNotFound() != null) // may happen if you return null on selectionRange queries
            val rectAndRange = textInputClient.firstRectForCharacterRange(range)
            (rectAndRange.actualRange ?: TextRange.notFound).toNative(NativeFirstRectForCharacterRangeArgs.actual_range_out(args))
            rectAndRange.rect.toNative(NativeFirstRectForCharacterRangeArgs.first_rect_out(args))
        }
    }

    // called from native code
    private fun characterIndexForPointCallback(point: MemorySegment): Long {
        return ffiUpCall(defaultResult = 0L) {
            textInputClient.characterIndexForPoint(LogicalPoint.fromNative(point)) ?: TextInputContext.notFoundOffset
        }
    }

    // called from native code
    private fun doCommandCallback(command: MemorySegment): Boolean {
        return ffiUpCall(defaultResult = false) {
            textInputClient.doCommand(command.getUtf8String(0))
        }
    }

    internal fun toNative(): MemorySegment {
        val native = NativeTextInputClient.allocate(arena)
        NativeTextInputClient.has_marked_text(native, NativeHasMarkedTextCallback.allocate(this::hasMarkedTextCallback, arena))
        NativeTextInputClient.marked_range(native, NativeMarkedRangeCallback.allocate(this::markedRangeCallback, arena))
        NativeTextInputClient.selected_range(native, NativeSelectedRangeCallback.allocate(this::selectedRangeCallback, arena))

        NativeTextInputClient.set_marked_text(native, NativeSetMarkedTextCallback.allocate(this::setMarkedTextCallback, arena))
        NativeTextInputClient.unmark_text(native, NativeUnmarkTextCallback.allocate(this::unmarkTextCallback, arena))

        NativeTextInputClient.attributed_string_for_range(
            native,
            NativeAttributedStringForRangeCallback.allocate(this::attributedStringForRangeCallback, arena),
        )
        NativeTextInputClient.free_attributed_string_for_range(
            native,
            NativeFreeAttributedStringCallback.allocate(this::freeAttributedStringForRangeCallback, arena),
        )

        NativeTextInputClient.insert_text(native, NativeInsertTextCallback.allocate(this::insertTextCallback, arena))
        NativeTextInputClient.first_rect_for_character_range(
            native,
            NativeFirstRectForCharacterRangeCallback.allocate(this::firstRectForCharacterRangeCallback, arena),
        )
        NativeTextInputClient.character_index_for_point(
            native,
            NativeFirstRectForCharacterRangeCallback.allocate(this::characterIndexForPointCallback, arena),
        )
        NativeTextInputClient.do_command(native, NativeDoCommandCallback.allocate(this::doCommandCallback, arena))

        return native
    }

    override fun close() {
        arena.close()
    }
}
