package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeOnInsertText
import org.jetbrains.desktop.macos.generated.NativeTextInputClient
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object TextInputContext {
    public fun handleCurrentEvent(): Boolean {
        return desktop_macos_h.text_input_context_handle_current_event()
    }
}

public interface TextInputClient {
    public fun insertText(text: String)
    public fun doCommand(command: String)
}

public data class TextInputClientHolder(var textInputClient: TextInputClient?): AutoCloseable {
    private val arena = Arena.ofShared()

    // called from native code
    private fun onInsertText(text: MemorySegment) {
        ffiUpCall {
            textInputClient?.insertText(text.getUtf8String(0))
        }
    }

    // called from native code
    private fun onDoCommand(command: MemorySegment) {
        ffiUpCall {
            textInputClient?.doCommand(command.getUtf8String(0))
        }
    }

    internal fun toNative(): MemorySegment {
        val native = NativeTextInputClient.allocate(arena)
        NativeTextInputClient.on_insert_text(native, NativeOnInsertText.allocate(this::onInsertText, arena))
        NativeTextInputClient.on_do_command(native, NativeOnInsertText.allocate(this::onDoCommand, arena))
        return native
    }

    override fun close() {
        arena.close()
    }
}