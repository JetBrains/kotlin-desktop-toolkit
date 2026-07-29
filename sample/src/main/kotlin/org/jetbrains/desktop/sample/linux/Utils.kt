package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalRect
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.skia.Color
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.FontStyle
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import org.jetbrains.skia.paragraph.DecorationLineStyle
import org.jetbrains.skia.paragraph.DecorationStyle
import org.jetbrains.skia.paragraph.FontCollection
import org.jetbrains.skia.paragraph.Paragraph
import org.jetbrains.skia.paragraph.ParagraphBuilder
import org.jetbrains.skia.paragraph.ParagraphStyle
import org.jetbrains.skia.paragraph.TextStyle
import java.text.CharacterIterator
import kotlin.math.ceil
import kotlin.math.roundToInt

internal class TextLineCreator {
    companion object {
        val fontCollection = FontCollection().also {
            it.setDefaultFontManager(FontMgr.default)
        }

        fun defaultStyle(fontSize: Float?, color: Int): TextStyle {
            val textStyle = TextStyle()
            fontSize?.let {
                textStyle.setFontSize(it)
            }
            textStyle.setFontStyle(FontStyle.NORMAL)
            textStyle.setFontFamilies(arrayOf("sans-serif", "Arial"))
            textStyle.setColor(Color.WHITE)
            val paint = Paint()
            paint.color = color
            textStyle.setForeground(paint)
            return textStyle
        }

        fun preeditStyle(): TextStyle {
            val color = Color.WHITE
            val textStyle = defaultStyle(null, color)
            textStyle.setFontStyle(FontStyle.NORMAL)

            val decorationStyle = DecorationStyle(
                _underline = true,
                _overline = false,
                _lineThrough = false,
                _gaps = false,
                color = Color.GREEN,
                lineStyle = DecorationLineStyle.SOLID,
                thicknessMultiplier = 1f,
            )
            textStyle.setDecorationStyle(decorationStyle)

            return textStyle
        }

        fun selectionStyle(fontSize: Float, color: Int): TextStyle {
            val textStyle = defaultStyle(fontSize, color)
            val backgroundPaint = Paint()
            backgroundPaint.color = Color.BLUE
            textStyle.setBackground(backgroundPaint)
            return textStyle
        }
    }

    private var cachedParagraph: Paragraph? = null
    private var lastTextEntries: List<Pair<String, TextStyle?>>? = null
    private var lastFontSize: Float? = null
    private var lastColor: Int? = null
    private var lastWidth: Float? = null

    fun makeTextLine(textEntries: List<Pair<String, TextStyle?>>, fontSize: Float, color: Int, width: Float): Paragraph {
        val cachedParagraph = cachedParagraph
        val paragraph = if (cachedParagraph != null && lastTextEntries == textEntries && lastFontSize == fontSize && lastColor == color) {
            cachedParagraph
        } else {
            Logger.info { "makeTextLine for $textEntries" }
            val paragraphBuilder = ParagraphBuilder(ParagraphStyle(), fontCollection)
            paragraphBuilder.pushStyle(defaultStyle(fontSize, color))
            for ((text, style) in textEntries) {
                if (style != null) {
                    paragraphBuilder.pushStyle(style)
                }
                paragraphBuilder.addText(text)
                if (style != null) {
                    paragraphBuilder.popStyle()
                }
            }
            val paragraph = paragraphBuilder.build()
            this.cachedParagraph = paragraph
            lastTextEntries = textEntries
            lastFontSize = fontSize
            lastColor = color
            lastWidth = null
            paragraph
        }

        if (lastWidth != width) {
            lastWidth = width
            paragraph.layout(width)
        }
        return paragraph
    }

    fun makeTextLine(text: String, fontSize: Float, color: Int, width: Float): Paragraph {
        return makeTextLine(listOf(Pair(text, null)), fontSize, color, width)
    }
}

internal fun Paragraph.toLogicalSize(scale: Double): LogicalSize {
    return LogicalSize(
        width = ceil(maxIntrinsicWidth).toLogicalPixels(scale).roundToInt(),
        height = ceil(height).toLogicalPixels(scale).roundToInt(),
    )
}

internal fun Float.toLogicalPixels(scale: Double): LogicalPixels {
    return this / scale
}

internal data class LogicalDoubleRect(
    val x: LogicalPixels,
    val y: LogicalPixels,
    val width: LogicalPixels,
    val height: LogicalPixels,
) {
    companion object {
        val Zero = LogicalDoubleRect(0.0, 0.0, 0.0, 0.0)
    }
    fun contains(p: LogicalPoint): Boolean {
        return p.x > x &&
            p.x < (x + width) &&
            p.y > y &&
            p.y < (y + height)
    }

    internal fun toSkiko(scale: Double): Rect {
        return Rect.makeXYWH((x * scale).toFloat(), (y * scale).toFloat(), (width * scale).toFloat(), (height * scale).toFloat())
    }

    internal fun round(): LogicalRect {
        return LogicalRect(x.roundToInt(), y.roundToInt(), width.roundToInt(), height.roundToInt())
    }
}

internal fun LogicalPixels.toSkiko(scale: Double): Float {
    return (this * scale).toFloat()
}

internal data class StringBuilderCharacterIterator(private val text: StringBuilder) : CharacterIterator {
    private val begin: Int = 0
    private val end: Int = text.length

    // invariant: begin <= pos <= end
    private var pos: Int = 0

    override fun first(): Char {
        pos = begin
        return current()
    }

    override fun last(): Char {
        pos = if (end != begin) {
            end - 1
        } else {
            end
        }
        return current()
    }

    override fun setIndex(p: Int): Char {
        if (p !in begin..end) {
            throw IllegalArgumentException("Invalid index")
        }
        pos = p
        return current()
    }

    override fun current(): Char {
        return if (pos in begin..<end) {
            text[pos]
        } else {
            CharacterIterator.DONE
        }
    }

    override fun next(): Char {
        return if (pos < end - 1) {
            pos++
            text[pos]
        } else {
            pos = end
            CharacterIterator.DONE
        }
    }

    override fun previous(): Char {
        return if (pos > begin) {
            pos--
            text[pos]
        } else {
            CharacterIterator.DONE
        }
    }

    override fun getBeginIndex(): Int {
        return begin
    }

    override fun getEndIndex(): Int {
        return end
    }

    override fun getIndex(): Int {
        return pos
    }

    override fun clone(): Any {
        return copy()
    }
}
