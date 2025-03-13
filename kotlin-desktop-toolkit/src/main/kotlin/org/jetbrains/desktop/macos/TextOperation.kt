package org.jetbrains.desktop.macos

public sealed class TextOperation {
    public data class TextChanged(
        val windowId: WindowId,
        val text: String,
    ) : TextOperation()

    public data class TextCommand(
        val windowId: WindowId,
        val command: String,
    ) : TextOperation()

    public data class UnmarkText(val windowId: WindowId) : TextOperation()

    public data class SetMarkedText(val windowId: WindowId) : TextOperation()

    public fun windowId(): WindowId {
        return when (this) {
            is TextCommand -> windowId
            is TextChanged -> windowId
            is UnmarkText -> windowId
            is SetMarkedText -> windowId
        }
    }

    internal companion object
}
