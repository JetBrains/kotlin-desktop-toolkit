package org.jetbrains.desktop.sample.macos

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.Paint
import org.jetbrains.skia.PaintMode
import org.jetbrains.skia.RRect
import org.jetbrains.skia.paragraph.Alignment
import org.jetbrains.skia.paragraph.FontCollection
import org.jetbrains.skia.paragraph.ParagraphBuilder
import org.jetbrains.skia.paragraph.ParagraphStyle
import org.jetbrains.skia.paragraph.TextStyle

/**
 * A toy push button living in the ordinary client area, activated on mouse-down.
 *
 * The sample uses these to drive [ToyTextInput]'s IME state: one drops the window's IME session,
 * another discards an in-flight composition.
 *
 * Layout and hit-testing are in logical pixels (matching event coordinates), while drawing happens
 * in physical pixels (matching the Skia canvas).
 */
class ToyButton(
    private val label: String,
    var origin: LogicalPoint,
    var size: LogicalSize,
    private val onClick: () -> Unit,
) {
    private val fontCollection = FontCollection().apply { setDefaultFontManager(FontMgr.default) }
    private var hovered = false

    fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.MouseMoved -> {
                hovered = hitTest(event.locationInWindow)
                EventHandlerResult.Continue
            }

            is Event.MouseDown -> {
                if (hitTest(event.locationInWindow)) {
                    onClick()
                    EventHandlerResult.Stop
                } else {
                    EventHandlerResult.Continue
                }
            }

            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, scale: Double) {
        val sf = scale.toFloat()
        val left = (origin.x * scale).toFloat()
        val top = (origin.y * scale).toFloat()
        val right = left + (size.width * scale).toFloat()
        val bottom = top + (size.height * scale).toFloat()
        val rect = RRect.makeLTRB(left, top, right, bottom, 6f * sf)

        // Backplate
        Paint().use { paint ->
            paint.color = if (hovered) 0xFF3C3C3C.toInt() else 0xFF2A2A2A.toInt()
            paint.isAntiAlias = true
            canvas.drawRRect(rect, paint)
        }

        // Border
        Paint().use { paint ->
            paint.color = 0xFF555555.toInt()
            paint.mode = PaintMode.STROKE
            paint.strokeWidth = sf
            paint.isAntiAlias = true
            canvas.drawRRect(rect, paint)
        }

        // Label
        Paint().use { labelPaint ->
            labelPaint.color = 0xFFE0E0E0.toInt()
            labelPaint.isAntiAlias = true
            val textStyle = TextStyle().apply {
                setFontSize(13f * sf)
                setFontFamilies(arrayOf("Helvetica Neue", "sans-serif"))
                setForeground(labelPaint)
            }
            ParagraphBuilder(ParagraphStyle().apply { alignment = Alignment.CENTER }, fontCollection).use { builder ->
                builder.pushStyle(textStyle)
                builder.addText(label)
                builder.build().use { paragraph ->
                    paragraph.layout(right - left)
                    // `paint` draws from the top-left; center the laid-out paragraph vertically in the button.
                    paragraph.paint(canvas, left, (top + bottom) / 2f - paragraph.height / 2f)
                }
            }
        }
    }

    private fun hitTest(point: LogicalPoint): Boolean {
        return point.x in origin.x..(origin.x + size.width) &&
            point.y in origin.y..(origin.y + size.height)
    }
}
