package org.jetbrains.desktop.macos

public data class TextRange(
    val location: Long,
    val length: Long,
) {
    internal companion object
}

public data class GetSelectedRangeArgs(val windowId: WindowId) {
    internal companion object
}

public data class GetSelectedRangeResult(val range: TextRange)

public data class FirstRectForCharacterRangeArgs(
    val windowId: WindowId,
    val range: TextRange,
) {
    internal companion object
}

public data class FirstRectForCharacterRangeResult(
    val x: Double,
    val y: Double,
    val w: Double,
    val h: Double,
)

public interface TextContextHandler {
    public fun getSelectedRange(args: GetSelectedRangeArgs): GetSelectedRangeResult?
    public fun firstRectForCharacterRange(args: FirstRectForCharacterRangeArgs): FirstRectForCharacterRangeResult?
}

public sealed class TextOperation {
    public data class TextChanged(
        val windowId: WindowId,
        val text: String,
        val replacementRange: TextRange,
    ) : TextOperation()

    public data class TextCommand(
        val windowId: WindowId,
        val command: String,
    ) : TextOperation()

    public fun windowId(): WindowId {
        return when (this) {
            is TextCommand -> windowId
            is TextChanged -> windowId
        }
    }

    internal companion object
}
