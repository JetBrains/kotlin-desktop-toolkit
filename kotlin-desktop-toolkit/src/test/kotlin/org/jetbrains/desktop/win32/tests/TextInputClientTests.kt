package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.LogicalRect
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.TextInputClient
import org.jetbrains.desktop.win32.TextInputClientHolder
import org.jetbrains.desktop.win32.TextRange
import org.jetbrains.desktop.win32.UnderlineSegment
import org.jetbrains.desktop.win32.UnderlineStyle
import org.jetbrains.desktop.win32.fromNative
import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_UnderlineSegment
import org.jetbrains.desktop.win32.generated.NativeBorrowedUtf8
import org.jetbrains.desktop.win32.generated.NativeCaretRectArgs
import org.jetbrains.desktop.win32.generated.NativeEvent
import org.jetbrains.desktop.win32.generated.NativeInputLanguageChangedEvent
import org.jetbrains.desktop.win32.generated.NativeLogicalPoint
import org.jetbrains.desktop.win32.generated.NativeLogicalRect
import org.jetbrains.desktop.win32.generated.NativeLogicalSize
import org.jetbrains.desktop.win32.generated.NativeTextRange
import org.jetbrains.desktop.win32.generated.NativeUnderlineSegment
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import org.jetbrains.desktop.win32.readBorrowedUtf8
import org.jetbrains.desktop.win32.readUnderlines
import org.junit.jupiter.api.Test
import java.lang.foreign.Arena
import java.lang.foreign.ValueLayout
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertSame
import kotlin.test.assertTrue

class TextInputClientTests {
    private class RecordingClient : TextInputClient {
        var inserted = ""
        override fun selectedRange(): TextRange = TextRange(3, 0)
        override fun caretRect(range: TextRange): LogicalRect =
            LogicalRect(LogicalPoint(range.location.toFloat(), 7f), LogicalSize(1f, 18f))
        override fun insertText(text: String) {
            inserted += text
        }
        override fun setMarkedText(
            text: String,
            selectedRange: TextRange?,
            underlines: List<org.jetbrains.desktop.win32.UnderlineSegment>,
        ) = Unit
        override fun unmarkText() = Unit
        override fun discardMarkedText() = Unit
    }

    @Test
    fun `range sentinel uses location only`() = Arena.ofConfined().use { arena ->
        val native = NativeTextRange.allocate(arena)
        TextRange.notFound.toNative(native)
        assertEquals(null, TextRange.fromNative(native).nullIfNotFound())
        TextRange(5, 2).toNative(native)
        assertEquals(TextRange(5, 2), TextRange.fromNative(native).nullIfNotFound())
    }

    @Test
    fun `selected range callback writes caller storage`() = Arena.ofConfined().use { arena ->
        val holder = TextInputClientHolder()
        val client = RecordingClient()
        holder.replace(client) { assertSame(TextInputClient.Noop, holder.textInputClient) }
        val native = NativeTextRange.allocate(arena)
        holder.selectedRangeCallback(native)
        assertEquals(TextRange(3, 0), TextRange.fromNative(native))
        holder.close()
    }

    @Test
    fun `caret callback reads range and writes inline rect`() = Arena.ofConfined().use { arena ->
        val holder = TextInputClientHolder()
        holder.replace(RecordingClient()) {}
        val args = NativeCaretRectArgs.allocate(arena)
        TextRange(9, 0).toNative(NativeCaretRectArgs.range_in(args))
        holder.caretRectCallback(args)
        val rect = NativeCaretRectArgs.rect_out(args)
        assertEquals(9f, NativeLogicalPoint.x(NativeLogicalRect.origin(rect)))
        assertEquals(7f, NativeLogicalPoint.y(NativeLogicalRect.origin(rect)))
        assertEquals(1f, NativeLogicalSize.width(NativeLogicalRect.size(rect)))
        assertEquals(18f, NativeLogicalSize.height(NativeLogicalRect.size(rect)))
        holder.close()
    }

    @Test
    fun `borrowed utf8 decoder preserves embedded nul`() = Arena.ofConfined().use { arena ->
        val bytes = byteArrayOf('A'.code.toByte(), 0, 'B'.code.toByte())
        val native = NativeBorrowedUtf8.allocate(arena)
        NativeBorrowedUtf8.ptr(native, arena.allocateFrom(ValueLayout.JAVA_BYTE, *bytes))
        NativeBorrowedUtf8.len(native, bytes.size.toLong())
        assertEquals("A\u0000B", readBorrowedUtf8(native))
    }

    @Test
    fun `underline decoder reads range style and target`() = Arena.ofConfined().use { arena ->
        val items = NativeUnderlineSegment.allocateArray(1L, arena)
        val item = NativeUnderlineSegment.asSlice(items, 0L)
        NativeTextRange.location(NativeUnderlineSegment.range(item), 2)
        NativeTextRange.length(NativeUnderlineSegment.range(item), 3)
        NativeUnderlineSegment.style(item, desktop_win32_h.NativeUnderlineStyle_Thick())
        NativeUnderlineSegment.target_clause(item, true)
        val borrowed = NativeBorrowedArray_UnderlineSegment.allocate(arena)
        NativeBorrowedArray_UnderlineSegment.ptr(borrowed, items)
        NativeBorrowedArray_UnderlineSegment.len(borrowed, 1)
        assertEquals(
            listOf(UnderlineSegment(TextRange(2, 3), UnderlineStyle.Thick, true)),
            readUnderlines(borrowed),
        )
    }

    @Test
    fun `replace and clear switch recipient only after native success`() {
        val holder = TextInputClientHolder()
        val first = RecordingClient()
        val second = RecordingClient()
        holder.replace(first) { assertSame(TextInputClient.Noop, holder.textInputClient) }
        holder.replace(second) { assertSame(first, holder.textInputClient) }
        holder.clear { assertSame(second, holder.textInputClient) }
        assertSame(TextInputClient.Noop, holder.textInputClient)
        holder.close()
    }

    @Test
    fun `holder owns one table until idempotent close`() {
        val holder = TextInputClientHolder()
        val table = holder.native
        assertTrue(table.scope().isAlive)
        holder.close()
        assertFalse(table.scope().isAlive)
        holder.close()
    }

    @Test
    fun `input language payload decodes hkl and locale`() = Arena.ofConfined().use { arena ->
        val eventStorage = NativeEvent.allocate(arena)
        NativeEvent.tag(eventStorage, desktop_win32_h.NativeEvent_InputLanguageChanged())
        val native = NativeEvent.input_language_changed(eventStorage)
        NativeInputLanguageChangedEvent.hkl(native, 0x0411L)
        NativeInputLanguageChangedEvent.locale_name(native, arena.allocateFrom("ja-JP"))
        val event = assertIs<Event.InputLanguageChanged>(Event.fromNative(eventStorage))
        assertEquals(0x0411L, event.hkl)
        assertEquals("ja-JP", event.localeName)
    }
}
