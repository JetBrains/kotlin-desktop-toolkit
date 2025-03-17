package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.*
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class InsertTextArgs(
    public val text: String,
)

public class SetMarkedTextArgs(
    public val text: String,
    public val selectedRange: TextRange,
    public val replacementRange: TextRange,
)

public interface TextInputClient {
    public fun insertText(args: InsertTextArgs)
    public fun doCommand(command: String): Boolean
    public fun hasMarkedText(): Boolean

    public fun markedRange(): TextRange?
    public fun unmarkText()
    public fun setMarkedText(args: SetMarkedTextArgs)
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

    internal fun modifyNative(nativeRange: MemorySegment) {
        NativeTextRange.location(nativeRange, location)
        NativeTextRange.length(nativeRange, length)
    }
}


public data class TextInputClientHolder(var textInputClient: TextInputClient?): AutoCloseable {
    private val arena = Arena.ofShared()

    // called from native code
    private fun onInsertText(s: MemorySegment) {
        ffiUpCall {
            val text = NativeOnInsertTextArgs.text(s).getUtf8String(0)
            textInputClient?.insertText(InsertTextArgs(text))
        }
    }

    // called from native code
    private fun onDoCommand(command: MemorySegment): Boolean {
        return ffiUpCall(defaultResult = false) {
            textInputClient?.doCommand(command.getUtf8String(0)) ?: false
        }
    }

    // called from native code
    private fun onHasMarkedText(): Boolean {
        return ffiUpCall(defaultResult = false) {
            textInputClient?.hasMarkedText() ?: false
        }
    }

    // called from native code
    private fun onMarkedRange(rangeOut: MemorySegment) {
        ffiUpCall(defaultResult = null) {
            val result = textInputClient?.markedRange()
            if (result != null) {
                NativeOptionalTextRange.exists(rangeOut, true)
                val nativeRange = NativeOptionalTextRange.range(rangeOut)
                result.modifyNative(nativeRange)
            } else {
                NativeOptionalTextRange.exists(rangeOut, true)
            }
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

    internal fun toNative(): MemorySegment {
        val native = NativeTextInputClient.allocate(arena)
        NativeTextInputClient.on_insert_text(native, NativeOnInsertText.allocate(this::onInsertText, arena))
        NativeTextInputClient.on_do_command(native, NativeOnInsertText.allocate(this::onDoCommand, arena))
        NativeTextInputClient.on_has_marked_text(native, NativeOnHasMarkedText.allocate(this::onHasMarkedText, arena))
        NativeTextInputClient.on_marked_range(native, NativeOnMarkedRange.allocate(this::onMarkedRange, arena))
        NativeTextInputClient.on_unmark_text(native, NativeOnUnmarkText.allocate(this::onUnmarkText, arena))
        NativeTextInputClient.on_set_marked_text(native, NativeOnSetMarkedText.allocate(this::onSetMarkedText, arena))
        return native
    }

    override fun close() {
        arena.close()
    }
}