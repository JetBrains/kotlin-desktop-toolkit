package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.LogicalPoint
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Paint
import org.jetbrains.skia.PaintMode
import org.jetbrains.skia.PaintStrokeCap
import org.jetbrains.skia.Rect

enum class CaptionButtonKind {
    Minimize,
    Maximize,
    Close,
}

/**
 * 1. Known problems // FIXED
 * Caption button might stick in pressed state, STR
 * -> press maximize button and hold
 * -> wait for snap layout popup
 * -> move mouse over popup
 * -> release mouse button
 *
 * 2. When one button is pressed and hold other one shouldn't be hoverable // FIXED
 *
 * 3. When Caption button is pressed, cursor lives window and released it stuck in hovered state // FIXED
 *
 * 4. When cursor is moved outside a button in pressed state button shouldn't be hovered // FIXED
 */

/**
 * Which caption buttons to show and enable, derived from the window's capabilities
 * ([org.jetbrains.desktop.win32.Window.isMinimizable] / [org.jetbrains.desktop.win32.Window.isMaximizable]).
 *
 * Mirrors the toolkit's own rule for system / custom title bars: Close is always shown and
 * enabled; Minimize and Maximize are shown together whenever either is allowed, and each is
 * disabled (visible but greyed, and not interactive) when its capability is off.
 */
class CaptionButtonModel(
    private val minimizable: Boolean,
    private val maximizable: Boolean,
) {
    fun isVisible(kind: CaptionButtonKind): Boolean = when (kind) {
        CaptionButtonKind.Close -> true
        CaptionButtonKind.Minimize, CaptionButtonKind.Maximize -> minimizable || maximizable
    }

    fun isEnabled(kind: CaptionButtonKind): Boolean = when (kind) {
        CaptionButtonKind.Minimize -> minimizable
        CaptionButtonKind.Maximize -> maximizable
        CaptionButtonKind.Close -> true
    }
}

private enum class ButtonState {
    Idle,
    Hovered,
    Pressed,
}

/**
 * A minimal caption-button strip (Minimize / Maximize-Restore / Close) drawn by the
 * sample itself, for a [org.jetbrains.desktop.win32.WindowTitleBarKind.None] window
 * that has no system-drawn buttons.
 *
 * The buttons are reported to Windows as the `HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE`
 * non-client regions in `WM_NCHITTEST` via `NCHitTest.setHitTestResult`, which gives the
 * maximize button the Win11 Snap Layouts flyover on hover. Press / hover state and
 * activation (on release) are tracked here and driven by the resulting pointer events;
 * the press is consumed by the sample so it never reaches the wedging `DefWindowProc`
 * non-client button loop.
 *
 * Visibility and enablement come from a [CaptionButtonModel] passed in per interaction /
 * frame, so the strip reflects the window's current capabilities. Layout and hit-testing
 * live in logical pixels (matching pointer-event coordinates); drawing happens in physical
 * pixels (matching the Skia canvas), scaled on the fly.
 */
class CaptionButtonsBar {
    private companion object {
        // Windows standard caption-button metrics at 100% scale.
        const val BUTTON_WIDTH = 46f // logical px
        const val BUTTON_HEIGHT = 32f // logical px
        const val GLYPH_HALF = 5f // half glyph extent, logical px
    }

    var hovered: CaptionButtonKind? = null
        private set
    var pressed: CaptionButtonKind? = null
        private set

    /** Height (logical px) of the title-bar band the buttons occupy — i.e. the draggable caption region. */
    val titleBarHeight: Float get() = BUTTON_HEIGHT

    /** Left edge (logical px) of [kind], anchored to the right edge of a window [width] wide. */
    private fun left(kind: CaptionButtonKind, width: Float): Float = when (kind) {
        CaptionButtonKind.Close -> width - BUTTON_WIDTH
        CaptionButtonKind.Maximize -> width - BUTTON_WIDTH * 2
        CaptionButtonKind.Minimize -> width - BUTTON_WIDTH * 3
    }

    /** Positional hit, ignoring visibility/enablement (used to classify the NC hit-test). */
    fun hitTest(point: LogicalPoint, width: Float): CaptionButtonKind? {
        if (point.y !in 0f..BUTTON_HEIGHT) return null
        return when {
            point.x < width - BUTTON_WIDTH * 3 || point.x > width -> null
            point.x >= width - BUTTON_WIDTH -> CaptionButtonKind.Close
            point.x >= width - BUTTON_WIDTH * 2 -> CaptionButtonKind.Maximize
            else -> CaptionButtonKind.Minimize
        }
    }

    /** Positional hit filtered to an interactive (visible + enabled) button, else null. */
    private fun interactiveHit(point: LogicalPoint, width: Float, model: CaptionButtonModel): CaptionButtonKind? =
        hitTest(point, width)?.takeIf { model.isVisible(it) && model.isEnabled(it) }

    /**
     * Updates hover state for a pointer move. Returns true if the visual state changed.
     *
     * Matches the system caption buttons: while a press is active (a button is held), only the
     * pressed button reacts, and only while the cursor is actually over it — other buttons never
     * hover, and the pressed button drops back to rest once the cursor leaves it. With no press
     * active, the button under the cursor hovers normally.
     */
    fun onPointerMove(point: LogicalPoint, width: Float, model: CaptionButtonModel): Boolean {
        val pressedKind = pressed
        val newHover = if (pressedKind != null) {
            pressedKind.takeIf { hitTest(point, width) == it }
        } else {
            interactiveHit(point, width, model)
        }
        if (newHover == hovered) return false
        hovered = newHover
        return true
    }

    fun onPointerExit(): Boolean {
        if (hovered == null) return false
        hovered = null
        return true
    }

    /**
     * Abandons an in-progress press *without* activating the button — used when the release was lost
     * (e.g. swallowed by the Win11 Snap Layouts flyover, so no `PointerUp` ever arrives). Returns true
     * if the visual state changed and a redraw is needed.
     */
    fun cancelPress(): Boolean {
        if (pressed == null) return false
        pressed = null
        return true
    }

    /** Begins a press; returns true if an interactive button was hit (the press should be consumed). */
    fun onPointerDown(point: LogicalPoint, width: Float, model: CaptionButtonModel): Boolean {
        val hit = interactiveHit(point, width, model)
        pressed = hit
        hovered = hit
        return hit != null && model.isEnabled(hit)
    }

    /** Ends a press; returns the activated button when the release lands on the pressed one. */
    fun onPointerUp(point: LogicalPoint, width: Float, model: CaptionButtonModel): CaptionButtonKind? {
        val wasPressed = pressed
        pressed = null
        val releasedOver = interactiveHit(point, width, model)
        hovered = releasedOver
        return if (wasPressed != null && wasPressed == releasedOver) wasPressed else null
    }

    fun draw(canvas: Canvas, size: PhysicalSize, scale: Float, isDark: Boolean, isMaximized: Boolean, model: CaptionButtonModel) {
        val widthLogical = size.width / scale
        for (kind in CaptionButtonKind.entries) {
            if (!model.isVisible(kind)) continue
            drawButton(canvas, kind, widthLogical, scale, isDark, isMaximized, model.isEnabled(kind))
        }
    }

    private fun drawButton(
        canvas: Canvas,
        kind: CaptionButtonKind,
        widthLogical: Float,
        scale: Float,
        isDark: Boolean,
        isMaximized: Boolean,
        enabled: Boolean,
    ) {
        val l = left(kind, widthLogical) * scale
        val r = l + BUTTON_WIDTH * scale
        val b = BUTTON_HEIGHT * scale
        // Disabled buttons never become hovered/pressed (see interactiveHit), so they always
        // render Idle and greyed. The pressed button shows its Pressed shade only while the cursor
        // is over it (`hovered` tracks that during a press, see onPointerMove); otherwise it falls
        // back to Idle, and while any press is active other buttons never show Hovered.
        val state = when {
            !enabled -> ButtonState.Idle
            pressed == kind -> if (hovered == kind) ButtonState.Pressed else ButtonState.Idle
            pressed == null && hovered == kind -> ButtonState.Hovered
            else -> ButtonState.Idle
        }

        val backplate = backplateColor(kind, state, isDark)
        if (backplate != 0) {
            Paint().use { paint ->
                paint.color = backplate
                paint.mode = PaintMode.FILL
                canvas.drawRect(Rect.makeLTRB(l, 0f, r, b), paint)
            }
        }

        Paint().use { paint ->
            paint.color = glyphColor(kind, state, isDark, enabled)
            paint.mode = PaintMode.STROKE
            paint.strokeWidth = scale // 1 logical px
            paint.strokeCap = PaintStrokeCap.BUTT
            paint.isAntiAlias = true

            val cx = (l + r) / 2f
            val cy = b / 2f
            val h = GLYPH_HALF * scale
            when (kind) {
                CaptionButtonKind.Minimize -> canvas.drawLine(cx - h, cy, cx + h, cy, paint)
                CaptionButtonKind.Maximize -> if (isMaximized) {
                    // Restore glyph: two overlapping squares.
                    val o = 2f * scale
                    canvas.drawRect(Rect.makeLTRB(cx - h + o, cy - h - o, cx + h + o, cy + h - o), paint)
                    canvas.drawRect(Rect.makeLTRB(cx - h - o, cy - h + o, cx + h - o, cy + h + o), paint)
                } else {
                    canvas.drawRect(Rect.makeLTRB(cx - h, cy - h, cx + h, cy + h), paint)
                }
                CaptionButtonKind.Close -> {
                    canvas.drawLine(cx - h, cy - h, cx + h, cy + h, paint)
                    canvas.drawLine(cx - h, cy + h, cx + h, cy - h, paint)
                }
            }
        }
    }

    /** ARGB backplate fill, or 0 (transparent) when no backplate should be drawn. */
    private fun backplateColor(kind: CaptionButtonKind, state: ButtonState, isDark: Boolean): Int = when (kind) {
        CaptionButtonKind.Close -> when (state) {
            ButtonState.Idle -> 0
            ButtonState.Hovered -> 0xFFC42B1C.toInt()
            ButtonState.Pressed -> 0xCCC42B1C.toInt()
        }
        else -> when (state) {
            ButtonState.Idle -> 0
            ButtonState.Hovered -> if (isDark) 0x1AFFFFFF else 0x14000000
            ButtonState.Pressed -> if (isDark) 0x0FFFFFFF else 0x0A000000
        }
    }

    private fun glyphColor(kind: CaptionButtonKind, state: ButtonState, isDark: Boolean, enabled: Boolean): Int = when {
        !enabled -> if (isDark) 0x66FFFFFF else 0x661A1A1A // dimmed (≈40% alpha)
        kind == CaptionButtonKind.Close && state != ButtonState.Idle -> 0xFFFFFFFF.toInt()
        isDark -> 0xFFFFFFFF.toInt()
        else -> 0xFF1A1A1A.toInt()
    }
}
