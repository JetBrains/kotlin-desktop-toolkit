package org.jetbrains.desktop.macos

public sealed class TextOperation {
    public data class TextChanged(
        val windowId: WindowId,
        val originalEvent: Event.KeyDown?,
        val text: String,
    ) : TextOperation()

    public data class TextCommand(
        val windowId: WindowId,
        val originalEvent: Event.KeyDown?,
        val command: String,
    ) : TextOperation()

    public fun windowId(): WindowId? {
        return when (this) {
            is TextCommand -> windowId
            is TextChanged -> windowId
            else -> null
        }
    }

    internal companion object
}
