package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeTextChangedOperation
import org.jetbrains.desktop.macos.generated.NativeTextCommandOperation
import org.jetbrains.desktop.macos.generated.NativeTextOperation
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.MemorySegment

public sealed class TextOperation {
    public data class TextChanged(
        val windowId: WindowId,
        val text: String,
    ) : TextOperation()

    public data class TextCommand(
        val windowId: WindowId,
        val command: String,
    ) : TextOperation()

    public fun windowId(): WindowId? {
        return when (this) {
            is TextCommand -> windowId
            is TextChanged -> windowId
            else -> null
        }
    }

    internal companion object {
        internal fun fromNative(s: MemorySegment): TextOperation {
            return when (NativeTextOperation.tag(s)) {
                desktop_macos_h.NativeTextOperation_TextChanged() -> {
                    val nativeEvent = NativeTextOperation.text_changed(s)
                    TextChanged(
                        windowId = NativeTextChangedOperation.window_id(nativeEvent),
                        text = NativeTextChangedOperation.text(nativeEvent).getUtf8String(0),
                    )
                }
                desktop_macos_h.NativeTextOperation_TextCommand() -> {
                    val nativeEvent = NativeTextOperation.text_command(s)
                    TextCommand(
                        windowId = NativeTextCommandOperation.window_id(nativeEvent),
                        command = NativeTextCommandOperation.command(nativeEvent).getUtf8String(0),
                    )
                }
                else -> {
                    error("Unexpected TextOperation tag")
                }
            }
        }
    }
}
