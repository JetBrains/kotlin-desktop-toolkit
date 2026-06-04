package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.FontStyle
import org.jetbrains.skia.Paint
import org.jetbrains.skia.PaintMode
import org.jetbrains.skia.RRect
import org.jetbrains.skia.paragraph.Alignment
import org.jetbrains.skia.paragraph.FontCollection
import org.jetbrains.skia.paragraph.ParagraphBuilder
import org.jetbrains.skia.paragraph.ParagraphStyle
import org.jetbrains.skia.paragraph.TextStyle

private enum class CounterButtonState {
    Idle,
    Hovered,
    Pressed,
}

/**
 * A simple client-area push button that increments a counter each time it is clicked.
 *
 * Unlike [CaptionButtonsBar] (which lives in the non-client title-bar band), this button
 * sits in the ordinary client area, anchored to the bottom-left corner so it is flush with
 * the left and bottom window borders. It receives normal `HTCLIENT` pointer events.
 *
 * Like the caption buttons, layout and hit-testing live in logical pixels (matching
 * pointer-event coordinates) while drawing happens in physical pixels (matching the Skia
 * canvas), scaled on the fly. State is driven by the pointer events the sample forwards;
 * activation happens on release when the pointer is still over the button.
 */
class CounterButton {
    private companion object {
        const val WIDTH = 120f // logical px
        const val HEIGHT = 40f // logical px

        val fontCollection = FontCollection().apply { setDefaultFontManager(FontMgr.default) }
    }

    var count: Int = 0
        private set

    private var hovered = false
    var pressed = false
        private set

    /** Button rect in logical pixels, anchored to the bottom-left of a [clientHeight]-tall client area. */
    private fun rect(clientHeight: Float): Rect4 {
        val top = clientHeight - HEIGHT
        return Rect4(0f, top, WIDTH, top + HEIGHT)
    }

    private fun hit(point: LogicalPoint, clientHeight: Float): Boolean {
        val r = rect(clientHeight)
        return point.x in r.left..r.right && point.y in r.top..r.bottom
    }

    /** Returns true if the visual state changed and a redraw is needed. */
    fun onPointerMove(point: LogicalPoint, clientHeight: Float): Boolean {
        val newHover = hit(point, clientHeight)
        if (newHover == hovered) return false
        hovered = newHover
        return true
    }

    fun onPointerExit(): Boolean {
        if (!hovered) return false
        hovered = false
        return true
    }

    /** Begins a press; returns true if the button was hit (so the press should be consumed). */
    fun onPointerDown(point: LogicalPoint, clientHeight: Float): Boolean {
        pressed = hit(point, clientHeight)
        hovered = pressed
        return pressed
    }

    /** Ends a press; increments the counter and returns true when the release lands on the pressed button. */
    fun onPointerUp(point: LogicalPoint, clientHeight: Float): Boolean {
        val wasPressed = pressed
        pressed = false
        val releasedOver = hit(point, clientHeight)
        hovered = releasedOver
        if (wasPressed && releasedOver) {
            count++
            return true
        }
        return false
    }

    fun draw(canvas: Canvas, size: PhysicalSize, scale: Float, isDark: Boolean) {
        val clientHeight = size.height / scale
        val r = rect(clientHeight)
        val l = r.left * scale
        val t = r.top * scale
        val rr = r.right * scale
        val b = r.bottom * scale
        val state = when {
            pressed -> CounterButtonState.Pressed
            hovered -> CounterButtonState.Hovered
            else -> CounterButtonState.Idle
        }

        val radius = 6f * scale
        val rect = RRect.makeLTRB(l, t, rr, b, radius)

        Paint().use { paint ->
            paint.color = backplateColor(state, isDark)
            paint.mode = PaintMode.FILL
            paint.isAntiAlias = true
            canvas.drawRRect(rect, paint)
        }
        Paint().use { paint ->
            paint.color = borderColor(isDark)
            paint.mode = PaintMode.STROKE
            paint.strokeWidth = scale // 1 logical px
            paint.isAntiAlias = true
            canvas.drawRRect(rect, paint)
        }

        Paint().use { labelPaint ->
            labelPaint.color = labelColor(isDark)
            labelPaint.isAntiAlias = true
            val textStyle = TextStyle().apply {
                setFontSize(14f * scale)
                setFontStyle(FontStyle.NORMAL)
                setFontFamilies(arrayOf("sans-serif", "Arial"))
                setForeground(labelPaint)
            }
            val paragraphStyle = ParagraphStyle().apply { alignment = Alignment.CENTER }
            ParagraphBuilder(paragraphStyle, fontCollection).use { builder ->
                builder.pushStyle(textStyle)
                builder.addText("Count: $count")
                builder.build().use { paragraph ->
                    paragraph.layout(rr - l)
                    // `paint` draws from the top-left; center the laid-out paragraph vertically in the button.
                    paragraph.paint(canvas, l, (t + b) / 2f - paragraph.height / 2f)
                }
            }
        }
    }

    /** Plain LTRB rectangle in logical pixels; avoids depending on a Skia/AWT rect type for layout math. */
    private data class Rect4(
        val left: Float,
        val top: Float,
        val right: Float,
        val bottom: Float,
    )

    private fun backplateColor(state: CounterButtonState, isDark: Boolean): Int = when (state) {
        CounterButtonState.Idle -> if (isDark) 0x14FFFFFF else 0x0F000000
        CounterButtonState.Hovered -> if (isDark) 0x26FFFFFF else 0x1A000000
        CounterButtonState.Pressed -> if (isDark) 0x40FFFFFF else 0x29000000
    }

    private fun borderColor(isDark: Boolean): Int = if (isDark) 0x33FFFFFF else 0x33000000

    private fun labelColor(isDark: Boolean): Int = if (isDark) 0xFFFFFFFF.toInt() else 0xFF1A1A1A.toInt()
}
