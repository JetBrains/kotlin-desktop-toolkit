package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.LogicalRect
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.TextCompositionAttribute
import org.jetbrains.desktop.win32.TextCompositionSegment
import org.jetbrains.desktop.win32.TextInputClient
import org.jetbrains.desktop.win32.TextInputClientHolder
import org.jetbrains.desktop.win32.TextRange
import org.jetbrains.desktop.win32.fromNative
import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_TextCompositionSegment
import org.jetbrains.desktop.win32.generated.NativeBorrowedUtf8
import org.jetbrains.desktop.win32.generated.NativeCaretRectArgs
import org.jetbrains.desktop.win32.generated.NativeEvent
import org.jetbrains.desktop.win32.generated.NativeInputLanguageChangedEvent
import org.jetbrains.desktop.win32.generated.NativeLogicalPoint
import org.jetbrains.desktop.win32.generated.NativeLogicalRect
import org.jetbrains.desktop.win32.generated.NativeLogicalSize
import org.jetbrains.desktop.win32.generated.NativeSetMarkedTextArgs
import org.jetbrains.desktop.win32.generated.NativeTextCompositionSegment
import org.jetbrains.desktop.win32.generated.NativeTextRange
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import org.jetbrains.desktop.win32.readBorrowedUtf8
import org.jetbrains.desktop.win32.readSegments
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertSame
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertInstanceOf
import java.lang.foreign.Arena
import java.lang.foreign.ValueLayout

class TextInputClientTests {
    private class RecordingClient : TextInputClient {
        var inserted = ""
        var markedSegments: List<TextCompositionSegment> = emptyList()
        override fun selectedRange(): TextRange = TextRange(3, 0)
        override fun caretRect(range: TextRange): LogicalRect =
            LogicalRect(LogicalPoint(range.location.toFloat(), 7f), LogicalSize(1f, 18f))
        override fun insertText(text: String) {
            inserted += text
        }
        override fun setMarkedText(text: String, selectedRange: TextRange?, segments: List<TextCompositionSegment>) {
            markedSegments = segments
        }
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
        holder.textInputClient = RecordingClient()
        val native = NativeTextRange.allocate(arena)
        holder.selectedRangeCallback(native)
        assertEquals(TextRange(3, 0), TextRange.fromNative(native))
        holder.close()
    }

    @Test
    fun `caret callback reads range and writes inline rect`() = Arena.ofConfined().use { arena ->
        val holder = TextInputClientHolder()
        holder.textInputClient = RecordingClient()
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
    fun `segment decoder reads every known attribute plus unspecified`() = Arena.ofConfined().use { arena ->
        val expected = listOf(
            TextCompositionAttribute.Input,
            TextCompositionAttribute.TargetConverted,
            TextCompositionAttribute.Converted,
            TextCompositionAttribute.TargetNotConverted,
            TextCompositionAttribute.InputError,
            TextCompositionAttribute.FixedConverted,
            TextCompositionAttribute.Unspecified,
        )
        val nativeAttributes = listOf(
            desktop_win32_h.NativeTextCompositionAttribute_Input(),
            desktop_win32_h.NativeTextCompositionAttribute_TargetConverted(),
            desktop_win32_h.NativeTextCompositionAttribute_Converted(),
            desktop_win32_h.NativeTextCompositionAttribute_TargetNotConverted(),
            desktop_win32_h.NativeTextCompositionAttribute_InputError(),
            desktop_win32_h.NativeTextCompositionAttribute_FixedConverted(),
            desktop_win32_h.NativeTextCompositionAttribute_Unspecified(),
        )
        val items = NativeTextCompositionSegment.allocateArray(expected.size.toLong(), arena)
        expected.indices.forEach { index ->
            val item = NativeTextCompositionSegment.asSlice(items, index.toLong())
            NativeTextRange.location(NativeTextCompositionSegment.range(item), index.toLong())
            NativeTextRange.length(NativeTextCompositionSegment.range(item), 1)
            NativeTextCompositionSegment.attribute(item, nativeAttributes[index])
        }
        val borrowed = NativeBorrowedArray_TextCompositionSegment.allocate(arena)
        NativeBorrowedArray_TextCompositionSegment.ptr(borrowed, items)
        NativeBorrowedArray_TextCompositionSegment.len(borrowed, expected.size.toLong())
        assertEquals(
            expected.mapIndexed { index, attribute -> TextCompositionSegment(TextRange(index.toLong(), 1), attribute) },
            readSegments(borrowed),
        )
    }

    @Test
    fun `set marked text copies reverse borrowed segments before upcall returns`() {
        val holder = TextInputClientHolder()
        val client = RecordingClient()
        holder.textInputClient = client
        Arena.ofConfined().use { arena ->
            val items = NativeTextCompositionSegment.allocateArray(1L, arena)
            val item = NativeTextCompositionSegment.asSlice(items, 0L)
            NativeTextRange.location(NativeTextCompositionSegment.range(item), 2)
            NativeTextRange.length(NativeTextCompositionSegment.range(item), 3)
            NativeTextCompositionSegment.attribute(item, desktop_win32_h.NativeTextCompositionAttribute_TargetConverted())
            val borrowed = NativeBorrowedArray_TextCompositionSegment.allocate(arena)
            NativeBorrowedArray_TextCompositionSegment.ptr(borrowed, items)
            NativeBorrowedArray_TextCompositionSegment.len(borrowed, 1)
            val args = NativeSetMarkedTextArgs.allocate(arena)
            NativeBorrowedUtf8.ptr(NativeSetMarkedTextArgs.text(args), arena.allocateFrom("preedit"))
            NativeBorrowedUtf8.len(NativeSetMarkedTextArgs.text(args), 7L)
            TextRange(0, 0).toNative(NativeSetMarkedTextArgs.selected_range(args))
            NativeSetMarkedTextArgs.segments(args, borrowed)
            holder.setMarkedTextCallback(args)
        }
        assertEquals(
            listOf(TextCompositionSegment(TextRange(2, 3), TextCompositionAttribute.TargetConverted)),
            client.markedSegments,
        )
        holder.close()
    }

    @Test
    fun `holder starts with the noop recipient`() {
        val holder = TextInputClientHolder()
        assertSame(TextInputClient.Noop, holder.textInputClient)
        holder.close()
    }

    @Test
    fun `holder owns one table until close`() {
        val holder = TextInputClientHolder()
        val table = holder.native
        assertTrue(table.scope().isAlive)
        holder.close()
        assertFalse(table.scope().isAlive)
    }

    @Test
    fun `input language payload decodes hkl and locale`() = Arena.ofConfined().use { arena ->
        val eventStorage = NativeEvent.allocate(arena)
        NativeEvent.tag(eventStorage, desktop_win32_h.NativeEvent_InputLanguageChanged())
        val native = NativeEvent.input_language_changed(eventStorage)
        NativeInputLanguageChangedEvent.hkl(native, 0x0411L)
        NativeInputLanguageChangedEvent.locale_name(native, arena.allocateFrom("ja-JP"))
        val event = assertInstanceOf<Event.InputLanguageChanged>(Event.fromNative(eventStorage))
        assertEquals(0x0411L, event.hkl)
        assertEquals("ja-JP", event.localeName)
    }
}
