package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.Keyboard
import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.LogicalRect
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.PointerButton
import org.jetbrains.desktop.win32.TextInputClient
import org.jetbrains.desktop.win32.TextRange
import org.jetbrains.desktop.win32.UnderlineSegment
import org.jetbrains.desktop.win32.UnderlineStyle
import org.jetbrains.desktop.win32.VirtualKey
import org.jetbrains.desktop.win32.Window
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.Paint
import org.jetbrains.skia.PaintMode
import org.jetbrains.skia.Rect
import org.jetbrains.skia.paragraph.FontCollection
import org.jetbrains.skia.paragraph.Paragraph
import org.jetbrains.skia.paragraph.ParagraphBuilder
import org.jetbrains.skia.paragraph.ParagraphStyle
import org.jetbrains.skia.paragraph.TextStyle
import kotlin.math.abs

class ToyTextInputWin32(
    private val window: Window,
    var origin: LogicalPoint,
    var size: LogicalSize,
) : TextInputClient,
    AutoCloseable {
    private val fontCollection = FontCollection().apply { setDefaultFontManager(FontMgr.default) }
    private val fontSize = 18f
    private val padding = 8f
    private var horizontalOffset = 0f
    private val buffer = StringBuilder()
    private var cursor = 0
    private var anchor = 0
    private var marked: TextRange? = null
    private var underlines: List<UnderlineSegment> = emptyList()
    private var compositionBackup: CompositionBackup? = null

    private data class CompositionBackup(
        val replacedText: String,
        val range: TextRange,
        val cursor: Int,
        val anchor: Int,
    )

    override fun selectedRange(): TextRange {
        val start = minOf(cursor, anchor)
        return TextRange(start.toLong(), (maxOf(cursor, anchor) - start).toLong())
    }

    override fun caretRect(range: TextRange): LogicalRect {
        val index = range.location.toInt().coerceIn(0, buffer.length)
        return LogicalRect(
            LogicalPoint(origin.x + padding + measurePrefix(index) - horizontalOffset, origin.y + padding),
            LogicalSize(1f, lineHeight()),
        )
    }

    override fun insertText(text: String) {
        val target = marked ?: selectedRange()
        replace(target, text)
        cursor = target.location.toInt() + text.length
        anchor = cursor
        clearCompositionMetadata()
        changed()
    }

    override fun setMarkedText(text: String, selectedRange: TextRange?, underlines: List<UnderlineSegment>) {
        val target = marked ?: this.selectedRange().also { range ->
            val start = range.location.toInt()
            compositionBackup = CompositionBackup(
                replacedText = buffer.substring(start, start + range.length.toInt()),
                range = range,
                cursor = cursor,
                anchor = anchor,
            )
        }
        replace(target, text)
        marked = TextRange(target.location, text.length.toLong())
        this.underlines = underlines
        val local = selectedRange ?: TextRange(text.length.toLong(), 0)
        anchor = target.location.toInt() + local.location.toInt()
        cursor = anchor + local.length.toInt()
        changed()
    }

    override fun unmarkText() {
        clearCompositionMetadata()
        changed()
    }

    override fun discardMarkedText() {
        val mark = marked
        val backup = compositionBackup
        if (mark != null && backup != null) {
            replace(mark, backup.replacedText)
            cursor = backup.cursor
            anchor = backup.anchor
        }
        clearCompositionMetadata()
        changed()
    }

    private fun replace(range: TextRange, text: String) {
        val start = range.location.toInt()
        buffer.replace(start, start + range.length.toInt(), text)
    }

    private fun clearCompositionMetadata() {
        marked = null
        underlines = emptyList()
        compositionBackup = null
    }

    private fun changed() {
        revealCaret()
        window.notifySelectionChanged()
        window.requestRedraw()
    }

    fun reflow(newSize: LogicalSize) {
        size = newSize
        revealCaret()
        window.notifyLayoutChanged()
        window.requestRedraw()
    }

    fun handleEvent(event: Event): EventHandlerResult = when (event) {
        is Event.KeyDown -> handleKeyDown(event)
        is Event.PointerDown -> {
            if (event.button != PointerButton.Left || !hitTest(event.locationInWindow)) {
                EventHandlerResult.Continue
            } else if (marked != null) {
                // Keep the native composition and its rollback snapshot coherent. The sample
                // deliberately ignores app-side selection edits until IMM commits or cancels it.
                EventHandlerResult.Stop
            } else {
                cursor = findIndex(event.locationInWindow.x - origin.x - padding + horizontalOffset)
                anchor = cursor
                changed()
                EventHandlerResult.Stop
            }
        }
        is Event.ScrollWheelX -> handleScroll(event.scrollingDelta, event.locationInWindow)
        is Event.ScrollWheelY -> handleScroll(event.scrollingDelta, event.locationInWindow)
        else -> EventHandlerResult.Continue
    }

    private fun handleKeyDown(event: Event.KeyDown): EventHandlerResult {
        if (Keyboard.getKeyState(VirtualKey.Control).isDown ||
            Keyboard.getKeyState(VirtualKey.Menu).isDown
        ) {
            return EventHandlerResult.Continue
        }
        if (marked != null) {
            // Let the IME consume arrows/deletion while a preedit exists; do not mutate the app
            // selection or clear `compositionBackup` behind the native state machine.
            event.translate()
            return EventHandlerResult.Stop
        }
        when (event.virtualKey) {
            VirtualKey.Left -> move(-1)
            VirtualKey.Right -> move(1)
            VirtualKey.Home -> moveTo(0)
            VirtualKey.End -> moveTo(buffer.length)
            VirtualKey.Back -> deleteBackward()
            VirtualKey.Delete -> deleteForward()
            else -> {
                event.translate()
                return EventHandlerResult.Stop
            }
        }
        changed()
        return EventHandlerResult.Stop
    }

    private fun move(delta: Int) = moveTo((cursor + delta).coerceIn(0, buffer.length))

    private fun moveTo(index: Int) {
        cursor = index
        anchor = index
    }

    private fun deleteBackward() {
        if (cursor != anchor) {
            replace(selectedRange(), "")
            moveTo(minOf(cursor, anchor))
        } else if (cursor > 0) {
            buffer.deleteCharAt(cursor - 1)
            moveTo(cursor - 1)
        }
    }

    private fun deleteForward() {
        if (cursor != anchor) {
            replace(selectedRange(), "")
            moveTo(minOf(cursor, anchor))
        } else if (cursor < buffer.length) {
            buffer.deleteCharAt(cursor)
        }
    }

    fun draw(canvas: Canvas, scale: Float) {
        val x = origin.x * scale
        val y = origin.y * scale
        val width = size.width * scale
        val height = size.height * scale
        Paint().use { paint ->
            paint.color = 0xFF_20_20_20.toInt()
            canvas.drawRect(Rect.makeXYWH(x, y, width, height), paint)
            paint.mode = PaintMode.STROKE
            paint.strokeWidth = scale
            paint.color = 0xFF_70_70_70.toInt()
            canvas.drawRect(Rect.makeXYWH(x, y, width, height), paint)
        }
        buildParagraph(buffer.toString(), scale, 0xFF_F0_F0_F0.toInt()).use { paragraph ->
            val textX = x + (padding - horizontalOffset) * scale
            val textY = y + (height - paragraph.height) / 2f
            Paint().use { paint ->
                if (cursor != anchor) {
                    val start = minOf(cursor, anchor)
                    val end = maxOf(cursor, anchor)
                    paint.color = 0xFF_26_4F_78.toInt()
                    canvas.drawRect(
                        Rect.makeLTRB(
                            textX + measurePrefix(start, scale),
                            textY,
                            textX + measurePrefix(end, scale),
                            textY + paragraph.height,
                        ),
                        paint,
                    )
                }
                paragraph.paint(canvas, textX, textY)
                val mark = marked
                if (mark != null) {
                    for (segment in underlines) {
                        val start = mark.location.toInt() + segment.range.location.toInt()
                        val end = start + segment.range.length.toInt()
                        val x1 = textX + measurePrefix(start, scale)
                        val x2 = textX + measurePrefix(end, scale)
                        Paint().use { paint ->
                            paint.color = if (segment.targetClause) 0xFF_FF_CC_33.toInt() else 0xFF_E0_E0_E0.toInt()
                            paint.strokeWidth = when (segment.style) {
                                UnderlineStyle.Solid -> 1f * scale
                                UnderlineStyle.Dotted -> 1f * scale
                                UnderlineStyle.Thick -> 2f * scale
                            }
                            if (segment.style == UnderlineStyle.Dotted) {
                                var x = x1
                                while (x < x2) {
                                    canvas.drawPoint(x, textY + paragraph.height - scale, paint)
                                    x += 3f * scale
                                }
                            } else {
                                val underlineY = textY + paragraph.height - scale
                                canvas.drawLine(x1, underlineY, x2, underlineY, paint)
                            }
                        }
                    }
                }
                val caretX = textX + measurePrefix(cursor, scale)
                paint.color = 0xFF_F0_F0_F0.toInt()
                paint.strokeWidth = 1.5f * scale
                canvas.drawLine(caretX, textY, caretX, textY + paragraph.height, paint)
            }
        }
    }

    private fun buildParagraph(text: String, scale: Float, color: Int): Paragraph {
        val style = TextStyle().apply {
            setFontSize(fontSize * scale)
            setFontFamilies(
                arrayOf(
                    "Segoe UI",
                    "Yu Gothic UI",
                    "Microsoft YaHei UI",
                    "Malgun Gothic",
                    "Nirmala UI",
                    "sans-serif",
                ),
            )
            setColor(color)
        }
        val paragraph = ParagraphBuilder(ParagraphStyle(), fontCollection).use { builder ->
            builder.pushStyle(style)
            builder.addText(text.ifEmpty { " " })
            builder.build()
        }
        paragraph.layout(Float.MAX_VALUE)
        return paragraph
    }

    private fun measurePrefix(index: Int, scale: Float = 1f): Float {
        val end = index.coerceIn(0, buffer.length)
        if (end == 0) return 0f
        return buildParagraph(buffer.substring(0, end), scale, 0).use { it.maxIntrinsicWidth }
    }

    private fun lineHeight(): Float = buildParagraph(" ", 1f, 0).use { it.height }

    private fun revealCaret() {
        val contentWidth = (size.width - 2f * padding).coerceAtLeast(0f)
        val caretX = measurePrefix(cursor)
        horizontalOffset = when {
            caretX < horizontalOffset -> caretX
            caretX > horizontalOffset + contentWidth -> caretX - contentWidth
            else -> horizontalOffset
        }.coerceIn(0f, maxOf(0f, measurePrefix(buffer.length) - contentWidth))
    }

    private fun handleScroll(delta: Int, point: LogicalPoint): EventHandlerResult {
        if (!hitTest(point)) return EventHandlerResult.Continue
        val contentWidth = (size.width - 2f * padding).coerceAtLeast(0f)
        val maxOffset = maxOf(0f, measurePrefix(buffer.length) - contentWidth)
        horizontalOffset = (horizontalOffset - delta / 4f).coerceIn(0f, maxOffset)
        window.notifyLayoutChanged()
        window.requestRedraw()
        return EventHandlerResult.Stop
    }

    private fun hitTest(point: LogicalPoint): Boolean =
        point.x in origin.x..(origin.x + size.width) && point.y in origin.y..(origin.y + size.height)

    private fun findIndex(x: Float): Int = (0..buffer.length).minBy { index ->
        abs(measurePrefix(index) - x)
    }

    override fun close(): Unit = Unit
}
