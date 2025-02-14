package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.*
import org.jetbrains.desktop.macos.generated.Event as NativeEvent
import java.lang.foreign.MemorySegment

sealed class TextOperation {
    data class TextChanged(
        val windowId: WindowId,
        val text: String,
    ): TextOperation()

    data class TextCommand(
        val windowId: WindowId,
        val command: String,
    ): TextOperation()

    fun windowId(): WindowId? {
        return when (this) {
            is TextCommand -> windowId
            is TextChanged -> windowId
            else -> null
        }
    }

    companion object {
        internal fun fromNative(s: MemorySegment): TextOperation {
            return when (NativeEvent.tag(s)) {
                desktop_macos_h.TextOperation_TextChanged() -> {
                    val nativeEvent = NativeEvent.key_up(s)
                    TextOperation.TextChanged(
                        windowId = TextChangedOperation.window_id(nativeEvent),
                        text = TextChangedOperation.text(nativeEvent).getUtf8String(0),
                    )
                }
                desktop_macos_h.TextOperation_TextCommand() -> {
                    val nativeEvent = NativeEvent.key_up(s)
                    TextOperation.TextCommand(
                        windowId = TextCommandOperation.window_id(nativeEvent),
                        command = TextCommandOperation.command(nativeEvent).getUtf8String(0),
                    )
                }
                else -> {
                    error("Unexpected TextOperation tag")
                }
            }
        }
    }
}
