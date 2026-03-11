package org.jetbrains.desktop.sample.gtk

import org.jetbrains.desktop.gtk.Logger
import org.jetbrains.desktop.gtk.PhysicalPoint
import org.jetbrains.desktop.gtk.TextInputPreeditAttribute
import org.jetbrains.desktop.gtk.TextInputPreeditUnderlineType
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.FontStyle
import org.jetbrains.skia.Paint
import org.jetbrains.skia.paragraph.DecorationLineStyle
import org.jetbrains.skia.paragraph.DecorationStyle
import org.jetbrains.skia.paragraph.FontCollection
import org.jetbrains.skia.paragraph.Paragraph
import org.jetbrains.skia.paragraph.ParagraphBuilder
import org.jetbrains.skia.paragraph.ParagraphStyle
import org.jetbrains.skia.paragraph.TextStyle

fun Canvas.withTranslated(point: PhysicalPoint, block: Canvas.() -> Unit) = let { canvas ->
    canvas.save()
    canvas.translate(point.x.toFloat(), point.y.toFloat())
    canvas.block()
    canvas.restore()
}

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

        fun preeditStyle(attributes: TextInputPreeditAttribute?): TextStyle {
            val color = if (attributes?.foregroundHighlight == true) Color.BLACK else Color.WHITE
            val textStyle = defaultStyle(null, color)
            val fontStyle = when (Pair(attributes?.bold, attributes?.italic)) {
                Pair(true, true) -> FontStyle.BOLD_ITALIC
                Pair(true, false) -> FontStyle.BOLD
                Pair(false, true) -> FontStyle.ITALIC
                else -> FontStyle.NORMAL
            }

            textStyle.setFontStyle(fontStyle)
            val decorationStyle = DecorationStyle(
                _underline = (attributes != null && attributes.underline != TextInputPreeditUnderlineType.None),
                _overline = false,
                _lineThrough = attributes?.strikethrough == true,
                _gaps = false,
                color = if (attributes?.underline == TextInputPreeditUnderlineType.Error) Color.RED else Color.GREEN,
                lineStyle = when (attributes?.underline) {
                    TextInputPreeditUnderlineType.Double -> DecorationLineStyle.DOUBLE
                    TextInputPreeditUnderlineType.Low -> DecorationLineStyle.WAVY
                    TextInputPreeditUnderlineType.Error -> DecorationLineStyle.DOTTED
                    else -> DecorationLineStyle.SOLID
                },
                thicknessMultiplier = 1f,
            )
            textStyle.setDecorationStyle(decorationStyle)
            if (attributes?.backgroundHighlight == true) {
                val backgroundPaint = Paint()
                backgroundPaint.color = 0xFF_DE_EA_FF.toInt() // light blue
                textStyle.setBackground(backgroundPaint)
            }
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

    private val cache = mutableMapOf<Pair<List<Pair<String, TextStyle?>>, Pair<Float, Int>>, Paragraph>()

    fun makeTextLine(textEntries: List<Pair<String, TextStyle?>>, fontSize: Float, color: Int): Paragraph {
        val cacheKey = Pair(textEntries, Pair(fontSize, color))
        return cache[cacheKey] ?: run {
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
            paragraph.layout(Float.MAX_VALUE)
            cache[cacheKey] = paragraph
            paragraph
        }
    }

    fun makeTextLine(text: String, fontSize: Float, color: Int): Paragraph {
        return makeTextLine(listOf(Pair(text, null)), fontSize, color)
    }
}
