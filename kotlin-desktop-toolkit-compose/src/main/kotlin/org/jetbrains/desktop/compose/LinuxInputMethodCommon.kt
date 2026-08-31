// Named after Noria's file of the same name, which both Linux backends share; ktlint would rather a
// file holding one class were named after the class.
@file:Suppress("ktlint:standard:filename")

package org.jetbrains.desktop.compose

/**
 * The text around the caret, in the three unit systems a Wayland or GTK input method involves.
 *
 * The protocols measure surrounding text in UTF-8 *bytes*, address the caret and the selection in
 * *code points*, and Compose counts in UTF-16 *chars*, so all three have to be kept apart; the
 * conversions here are the only place that has to know about it.
 *
 * Shared by both Linux backends: the arithmetic is identical, only the `TextInputContentPurpose` enum
 * each of them maps a `KeyboardType` onto is per-backend.
 */
internal data class LinuxTextInputSurroundingText(
    val text: String,
    /** UTF-16 offset of the caret, i.e. the selection's end. */
    val cursorOffset: Int,
    /** UTF-16 offset of the selection's other end. */
    val selectionStartOffset: Int,
) {
    fun cursorCodePointOffset(): UShort = text.codePointCount(0, cursorOffset).toUShort()

    fun selectionStartCodePointOffset(): UShort = text.codePointCount(0, selectionStartOffset).toUShort()

    /** How many code points before the caret the given number of UTF-8 bytes covers. */
    fun codePointsBefore(byteLength: Int): Int {
        var remainingBytes = byteLength
        var index = cursorOffset
        var codePoints = 0
        while (remainingBytes > 0 && index > 0) {
            val codePoint = text.codePointBefore(index)
            remainingBytes -= codePoint.utf8Length()
            index -= Character.charCount(codePoint)
            codePoints++
        }
        return codePoints
    }

    /** How many code points after the caret the given number of UTF-8 bytes covers. */
    fun codePointsAfter(byteLength: Int): Int {
        var remainingBytes = byteLength
        var index = cursorOffset
        var codePoints = 0
        while (remainingBytes > 0 && index < text.length) {
            val codePoint = text.codePointAt(index)
            remainingBytes -= codePoint.utf8Length()
            index += Character.charCount(codePoint)
            codePoints++
        }
        return codePoints
    }
}

private fun Int.utf8Length(): Int = when {
    this < 0x80 -> 1
    this < 0x800 -> 2
    this < 0x10000 -> 3
    else -> 4
}
