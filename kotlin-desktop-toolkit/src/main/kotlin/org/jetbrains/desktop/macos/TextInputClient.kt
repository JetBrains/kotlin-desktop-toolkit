package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeOnFirstRectForCharacterRange
import org.jetbrains.desktop.macos.generated.NativeOnFirstRectForCharacterRangeArgs
import org.jetbrains.desktop.macos.generated.NativeOnInsertText
import org.jetbrains.desktop.macos.generated.NativeOnInsertTextArgs
import org.jetbrains.desktop.macos.generated.NativeOnSetMarkedText
import org.jetbrains.desktop.macos.generated.NativeOnSetMarkedTextArgs
import org.jetbrains.desktop.macos.generated.NativeOnUnmarkText
import org.jetbrains.desktop.macos.generated.NativeRectangle
import org.jetbrains.desktop.macos.generated.NativeTextInputClient
import org.jetbrains.desktop.macos.generated.NativeTextRange
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class InsetTextArgs(public val text: String)

public class SetMarkedTextArgs(
    public val text: String,
    public val selectedRange: TextRange,
    public val replacementRange: TextRange,
)

public interface TextInputClient {
    public fun insertText(args: InsetTextArgs)
    public fun doCommand(command: String): Boolean
    public fun hasMarkedText(): Boolean
    public fun unmarkText()
    public fun setMarkedText(args: SetMarkedTextArgs)
    public fun firstRectForCharacterRange(range: TextRange): Pair<Rectangle, TextRange?>
}

public class TextRange(
    public val location: Long,
    public val length: Long,
) {
    internal companion object {
        internal fun fromNative(native: MemorySegment): TextRange {
            return TextRange(NativeTextRange.location(native), NativeTextRange.length(native))
        }
    }
}

public class Rectangle(
    public val x: Double,
    public val y: Double,
    public val w: Double,
    public val h: Double,
) {
    internal companion object {
        internal fun fromNative(native: MemorySegment): Rectangle {
            return Rectangle(
                x = NativeRectangle.x(native),
                y = NativeRectangle.y(native),
                w = NativeRectangle.w(native),
                h = NativeRectangle.h(native),
            )
        }
    }
}

public data class TextInputClientHolder(var textInputClient: TextInputClient?) : AutoCloseable {
    private val arena = Arena.ofShared()

    // called from native code
    private fun onInsertText(s: MemorySegment) {
        ffiUpCall {
            val text = NativeOnInsertTextArgs.text(s).getUtf8String(0)
            textInputClient?.insertText(InsetTextArgs(text))
        }
    }

    // called from native code
    private fun onDoCommand(command: MemorySegment): Boolean {
        return ffiUpCall(defaultResult = false) {
            textInputClient?.doCommand(command.getUtf8String(0)) ?: false
        }
    }

    // called from native code
    private fun onUnmarkText() {
        ffiUpCall {
            textInputClient?.unmarkText()
        }
    }

    // called from native code
    private fun onSetMarkedText(s: MemorySegment) {
        val text = NativeOnSetMarkedTextArgs.text(s).getUtf8String(0)
        val selectedRange = TextRange.fromNative(NativeOnSetMarkedTextArgs.selected_range(s))
        val replacementRange = TextRange.fromNative(NativeOnSetMarkedTextArgs.replacement_range(s))
        ffiUpCall {
            textInputClient?.setMarkedText(SetMarkedTextArgs(text, selectedRange = selectedRange, replacementRange = replacementRange))
        }
    }

    // called from native code
    private fun onFirstRectForCharacterRange(s: MemorySegment) {
        val range = TextRange.fromNative(NativeOnFirstRectForCharacterRangeArgs.range(s))
        ffiUpCall {
            textInputClient?.firstRectForCharacterRange(range)?.let { rectAndActualRange ->
                val rect = rectAndActualRange.first
                val nativeResult = NativeOnFirstRectForCharacterRangeArgs.result_out(s)
                NativeRectangle.x(nativeResult, rect.x)
                NativeRectangle.y(nativeResult, rect.y)
                NativeRectangle.w(nativeResult, rect.w)
                NativeRectangle.h(nativeResult, rect.h)
            }
        }
    }

    internal fun toNative(): MemorySegment {
        val native = NativeTextInputClient.allocate(arena)
        NativeTextInputClient.on_insert_text(native, NativeOnInsertText.allocate(this::onInsertText, arena))
        NativeTextInputClient.on_do_command(native, NativeOnInsertText.allocate(this::onDoCommand, arena))
        NativeTextInputClient.on_unmark_text(native, NativeOnUnmarkText.allocate(this::onUnmarkText, arena))
        NativeTextInputClient.on_set_marked_text(native, NativeOnSetMarkedText.allocate(this::onSetMarkedText, arena))
        NativeTextInputClient.on_first_rect_for_character_range(
            native,
            NativeOnFirstRectForCharacterRange.allocate(this::onFirstRectForCharacterRange, arena),
        )
        return native
    }

    override fun close() {
        arena.close()
    }
}
