package org.jetbrains.desktop.sample.macos

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.KeyCode
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalRect
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.TextInputClient
import org.jetbrains.desktop.macos.TextRange
import org.jetbrains.desktop.macos.Window
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
import kotlin.math.max
import kotlin.math.min

/**
 * A toy single-line text input with a reference [TextInputClient] implementation.
 *
 * Demonstrates:
 * - Basic text editing (insert, delete, cursor movement, selection)
 * - IME composing text (marked text) with visual underline
 * - Proper coordinate reporting for IME candidate window positioning
 * - Click-to-position cursor
 *
 * The text model keeps a single [StringBuilder] buffer where marked (composing) text
 * is stored inline. [markedRange] tracks which portion is composing text from the IME.
 * All ranges are in UTF-16 code units, matching macOS NSTextInputClient conventions.
 */
class ToyTextInput(
    private val window: Window,
    var origin: LogicalPoint,
    var size: LogicalSize,
) : TextInputClient,
    AutoCloseable {

    // ---- Text model ----
    private val buf = StringBuilder()
    private var marked: TextRange? = null
    private var cursor: Int = 0
    private var anchor: Int = 0 // selection anchor; equals cursor when no selection

    // ---- Rendering (logical pixels) ----
    private val fontSize = 18.0
    private val padding = 8.0
    private val fontCollection = FontCollection().apply { setDefaultFontManager(FontMgr.default) }

    // ==================== TextInputClient ====================

    override fun hasMarkedText(): Boolean = marked != null

    override fun markedRange(): TextRange? = marked

    override fun selectedRange(): TextRange {
        val s = min(cursor, anchor)
        return TextRange(s.toLong(), (max(cursor, anchor) - s).toLong())
    }

    override fun insertText(text: String, replacementRange: TextRange?) {
        val r = effectiveRange(replacementRange)
        replaceRange(r, text)
        cursor = r.location.toInt() + text.length
        anchor = cursor
        marked = null
        window.textInputContext.invalidateCharacterCoordinates()
    }

    override fun setMarkedText(text: String, selectedRange: TextRange?, replacementRange: TextRange?) {
        val r = effectiveRange(replacementRange)
        replaceRange(r, text)
        marked = if (text.isNotEmpty()) TextRange(r.location, text.length.toLong()) else null
        // selectedRange is relative to the beginning of the marked text
        val off = selectedRange?.location?.toInt() ?: text.length
        val len = selectedRange?.length?.toInt() ?: 0
        anchor = r.location.toInt() + off
        cursor = anchor + len
        window.textInputContext.invalidateCharacterCoordinates()
    }

    override fun unmarkText() {
        marked = null
    }

    override fun doCommand(command: String): Boolean {
        when (command) {
            "noop:" -> {}
            "insertNewline:", "insertLineBreak:" -> {} // single-line, ignore
            "deleteBackward:" -> {
                if (hasSelection()) {
                    deleteSelection()
                } else if (cursor > 0) {
                    buf.deleteCharAt(--cursor)
                    anchor = cursor
                }
            }
            "deleteForward:" -> {
                if (hasSelection()) {
                    deleteSelection()
                } else if (cursor < buf.length) {
                    buf.deleteCharAt(cursor)
                }
            }
            "moveLeft:" -> move(-1, extend = false)
            "moveRight:" -> move(1, extend = false)
            "moveLeftAndModifySelection:" -> move(-1, extend = true)
            "moveRightAndModifySelection:" -> move(1, extend = true)
            "moveToBeginningOfLine:", "moveToBeginningOfDocument:" -> {
                cursor = 0
                anchor = 0
            }
            "moveToEndOfLine:", "moveToEndOfDocument:" -> {
                cursor = buf.length
                anchor = buf.length
            }
            "moveToBeginningOfLineAndModifySelection:" -> cursor = 0
            "moveToEndOfLineAndModifySelection:" -> cursor = buf.length
            "selectAll:" -> {
                anchor = 0
                cursor = buf.length
            }
            // Word-level movement, deleteWordBackward:, etc. could be added here.
            else -> {
                Logger.info { "ToyTextInput: unhandled command '$command'" }
                return false
            }
        }
        window.textInputContext.invalidateCharacterCoordinates()
        return true
    }

    override fun attributedStringForRange(range: TextRange): TextInputClient.StringAndRange {
        val s = range.location.toInt().coerceIn(0, buf.length)
        val e = (s + range.length.toInt()).coerceIn(s, buf.length)
        return TextInputClient.StringAndRange(buf.substring(s, e), TextRange(s.toLong(), (e - s).toLong()))
    }

    override fun firstRectForCharacterRange(range: TextRange): TextInputClient.RectAndRange {
        val idx = range.location.toInt().coerceIn(0, buf.length)
        val charX = textX(idx)
        val lineH = lineHeight()

        // contentOrigin is in top-down screen coordinates (y=0 at top of screen).
        // The Rust side converts to macOS bottom-up coords, so we just add view-local offsets.
        val co = window.contentOrigin
        val screenX = co.x + origin.x + charX
        val screenY = co.y + origin.y + padding

        return TextInputClient.RectAndRange(
            LogicalRect(LogicalPoint(screenX, screenY), LogicalSize(10.0, lineH)),
            range,
        )
    }

    override fun characterIndexForPoint(point: LogicalPoint): Long? {
        val co = window.contentOrigin
        val localX = point.x - co.x - origin.x - padding
        return findCharIndexAtX(localX).toLong()
    }

    // ==================== Event handling ====================

    fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.KeyDown -> {
                // Let menu shortcuts (Cmd+key) pass through to the menu system
                if (event.mightHaveKeyEquivalent && event.modifiers.command) {
                    return EventHandlerResult.Continue
                }
                // Let the macOS text input system process the event first.
                // For printable keys this calls insertText(); for editing keys this calls doCommand().
                val result = window.textInputContext.handleCurrentEvent()
                if (result == EventHandlerResult.Stop) return result
                // Fallback: handle editing keys directly when the text input system didn't process them
                handleKeyDirectly(event)
            }
            is Event.MouseDown -> {
                if (hitTest(event.locationInWindow)) {
                    placeCursorNear(event.locationInWindow)
                    EventHandlerResult.Stop
                } else {
                    EventHandlerResult.Continue
                }
            }
            else -> EventHandlerResult.Continue
        }
    }

    private fun handleKeyDirectly(event: Event.KeyDown): EventHandlerResult {
        val extend = event.modifiers.shift
        val handled = when (event.keyCode) {
            KeyCode.LeftArrow -> {
                move(-1, extend)
                true
            }
            KeyCode.RightArrow -> {
                move(1, extend)
                true
            }
            KeyCode.Home -> {
                cursor = 0
                if (!extend) anchor = cursor
                true
            }
            KeyCode.End -> {
                cursor = buf.length
                if (!extend) anchor = cursor
                true
            }
            KeyCode.Delete -> {
                doCommand("deleteBackward:")
                true
            }
            KeyCode.ForwardDelete -> {
                doCommand("deleteForward:")
                true
            }
            else -> false
        }
        return if (handled) {
            window.textInputContext.invalidateCharacterCoordinates()
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }

    // ==================== Drawing ====================

    fun draw(canvas: Canvas, scale: Double) {
        val sf = scale.toFloat()
        val px = (origin.x * scale).toFloat()
        val py = (origin.y * scale).toFloat()
        val pw = (size.width * scale).toFloat()
        val ph = (size.height * scale).toFloat()
        val pad = (padding * scale).toFloat()
        val physFontSize = (fontSize * scale).toFloat()

        val text = buf.toString()
        val para = buildParagraph(text, physFontSize, 0xFFE0E0E0.toInt())
        val lineH = para.height

        val textX = px + pad
        val textY = py + (ph - lineH) / 2f // vertically center

        // Background
        Paint().use { p ->
            p.color = 0xFF1E1E1E.toInt()
            canvas.drawRect(Rect.makeXYWH(px, py, pw, ph), p)
        }

        // Border
        Paint().use { p ->
            p.color = 0xFF555555.toInt()
            p.mode = PaintMode.STROKE
            p.strokeWidth = sf
            canvas.drawRect(Rect.makeXYWH(px, py, pw, ph), p)
        }

        // Selection highlight
        if (hasSelection()) {
            val s = min(cursor, anchor)
            val e = max(cursor, anchor)
            val x1 = measureWidth(text.substring(0, s), physFontSize)
            val x2 = measureWidth(text.substring(0, e), physFontSize)
            Paint().use { p ->
                p.color = 0xFF264F78.toInt()
                canvas.drawRect(Rect.makeXYWH(textX + x1, textY, x2 - x1, lineH), p)
            }
        }

        // Marked text underline
        marked?.let { mr ->
            val ms = mr.location.toInt()
            val me = ms + mr.length.toInt()
            val x1 = measureWidth(text.substring(0, ms), physFontSize)
            val x2 = measureWidth(text.substring(0, me), physFontSize)
            Paint().use { p ->
                p.color = 0xFFFFFF00.toInt()
                p.strokeWidth = 2f * sf
                canvas.drawLine(textX + x1, textY + lineH, textX + x2, textY + lineH, p)
            }
        }

        // Text
        if (text.isNotEmpty()) {
            para.paint(canvas, textX, textY)
        }

        // Cursor
        val cx = textX + measureWidth(text.substring(0, cursor), physFontSize)
        Paint().use { p ->
            p.color = 0xFFFFFFFF.toInt()
            p.strokeWidth = 1.5f * sf
            canvas.drawLine(cx, textY, cx, textY + lineH, p)
        }

        // Placeholder
        if (buf.isEmpty() && marked == null) {
            val placeholder = buildParagraph("Type here\u2026", physFontSize, 0xFF666666.toInt())
            placeholder.paint(canvas, textX, textY)
        }
    }

    private fun buildParagraph(text: String, physFontSize: Float, color: Int): Paragraph {
        val style = TextStyle().apply {
            setFontSize(physFontSize)
            setFontFamilies(arrayOf("Helvetica Neue", "Hiragino Sans", "sans-serif"))
            setColor(color)
        }
        val para = ParagraphBuilder(ParagraphStyle(), fontCollection).run {
            pushStyle(style)
            addText(text.ifEmpty { " " }) // ensure non-empty for metrics
            build()
        }
        para.layout(Float.MAX_VALUE)
        return para
    }

    private fun measureWidth(text: String, physFontSize: Float): Float {
        if (text.isEmpty()) return 0f
        val para = buildParagraph(text, physFontSize, 0)
        return para.maxIntrinsicWidth
    }

    // ==================== Helpers ====================

    override fun close() {
    }

    private fun hasSelection() = cursor != anchor

    private fun effectiveRange(explicit: TextRange?): TextRange {
        return explicit
            ?: marked
            ?: if (hasSelection()) {
                TextRange(min(cursor, anchor).toLong(), abs(cursor - anchor).toLong())
            } else {
                TextRange(cursor.toLong(), 0)
            }
    }

    private fun replaceRange(range: TextRange, text: String) {
        val s = range.location.toInt()
        val e = s + range.length.toInt()
        buf.replace(s, e, text)
    }

    private fun deleteSelection() {
        val s = min(cursor, anchor)
        buf.delete(s, max(cursor, anchor))
        cursor = s
        anchor = s
    }

    private fun move(delta: Int, extend: Boolean) {
        cursor = (cursor + delta).coerceIn(0, buf.length)
        if (!extend) anchor = cursor
    }

    private fun textX(charIdx: Int): Double {
        return padding + measureWidth(buf.substring(0, charIdx), fontSize.toFloat()).toDouble()
    }

    private fun lineHeight(): Double {
        return buildParagraph(" ", fontSize.toFloat(), 0).height.toDouble()
    }

    private fun hitTest(point: LogicalPoint): Boolean {
        return point.x in origin.x..(origin.x + size.width) &&
            point.y in origin.y..(origin.y + size.height)
    }

    private fun findCharIndexAtX(localX: Double): Int {
        var best = 0
        var bestDist = Double.MAX_VALUE
        val logFontSize = fontSize.toFloat()
        for (i in 0..buf.length) {
            val w = measureWidth(buf.substring(0, i), logFontSize).toDouble()
            val d = abs(w - localX)
            if (d < bestDist) {
                bestDist = d
                best = i
            }
        }
        return best
    }

    private fun placeCursorNear(point: LogicalPoint) {
        val localX = point.x - origin.x - padding
        cursor = findCharIndexAtX(localX)
        anchor = cursor
    }
}
