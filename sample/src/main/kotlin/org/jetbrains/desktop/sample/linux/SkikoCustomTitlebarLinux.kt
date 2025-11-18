package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.DesktopTitlebarAction
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalRect
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.MouseButton
import org.jetbrains.desktop.linux.Timestamp
import org.jetbrains.desktop.linux.Window
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.Image
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import kotlin.math.pow
import kotlin.time.Duration
import kotlin.use

private fun LogicalPoint.isInsideCircle(center: LogicalPoint, radius: LogicalPixels): Boolean {
    val xDiff = this.x - center.x
    val yDiff = this.y - center.y
    return xDiff.pow(2) + yDiff.pow(2) <= radius.pow(2)
}

internal class SkikoCustomTitlebarLinux(
    private var origin: LogicalPoint,
    var size: LogicalSize,
    var buttonLayout: TitlebarLayout,
    val requestClose: () -> Unit,
) {
    private var rectangles = ArrayList<Pair<LogicalRect, WindowButtonType>>()
    private var lastHeaderMouseDownTime: Timestamp? = null
    private var lastMouseLocation: LogicalPoint? = null
    private var leftClickStartLocation: LogicalPoint? = null
    private var isDragging: Boolean = false

    private var titleTextLineCreator = TextLineCreator(cachedFontSize = 0f, cachedText = "")

    companion object {
        const val CUSTOM_TITLEBAR_HEIGHT: LogicalPixels = 55f
        const val BUTTON_LINE_WIDTH: LogicalPixels = 5f
        const val MOVE_RADIUS: LogicalPixels = 3f
        val COLOR_DARK_GRAY = Color.makeRGB(128, 128, 128)
        val COLOR_LIGHT_GRAY = Color.makeRGB(211, 211, 211)
        val BUTTON_SIZE = LogicalSize(CUSTOM_TITLEBAR_HEIGHT, CUSTOM_TITLEBAR_HEIGHT)

        val APP_ICON = Image.makeFromEncoded(jbIconBytes())
    }

    fun configure(event: Event.WindowConfigure, layout: TitlebarLayout) {
        size = LogicalSize(width = event.size.width, height = CUSTOM_TITLEBAR_HEIGHT)
        setLayout(layout)
    }

    fun toggleMaximize(window: Window, windowState: WindowState) {
        if (windowState.maximized) {
            window.unmaximize()
        } else {
            window.maximize()
        }
    }

    fun setLayout(layout: TitlebarLayout) {
        buttonLayout = layout
        rectangles.clear()
        buttonLayout.let {
            val buttonsLeftWidth = origin.x + (it.layoutLeft.size * BUTTON_SIZE.width)
            val buttonsRightWidth = it.layoutRight.size * BUTTON_SIZE.width
            val rect = LogicalRect(
                LogicalPoint(buttonsLeftWidth, origin.y),
                LogicalSize(size.width - buttonsRightWidth - buttonsLeftWidth, CUSTOM_TITLEBAR_HEIGHT),
            )
            rectangles.add(Pair(rect, WindowButtonType.Title))
            for ((i, button) in it.layoutLeft.withIndex()) {
                val rect = LogicalRect(LogicalPoint(i * BUTTON_SIZE.height, origin.y), BUTTON_SIZE)
                rectangles.add(Pair(rect, button))
            }
            for ((i, button) in it.layoutRight.withIndex()) {
                val rect = LogicalRect(
                    LogicalPoint(origin.x + size.width - ((it.layoutRight.size - i) * BUTTON_SIZE.width), origin.y),
                    BUTTON_SIZE,
                )
                rectangles.add(Pair(rect, button))
            }
        }
    }

    private fun executeTitlebarAction(
        action: DesktopTitlebarAction,
        window: Window,
        locationInWindow: LogicalPoint,
        windowState: WindowState,
    ) {
        when (action) {
            DesktopTitlebarAction.ToggleMaximize -> {
                toggleMaximize(window, windowState)
            }
            DesktopTitlebarAction.Minimize -> {
                window.minimize()
            }
            DesktopTitlebarAction.None -> {}
            DesktopTitlebarAction.Menu -> {
                window.showMenu(locationInWindow)
            }
        }
    }

    private fun executeWindowAction(
        windowButton: WindowButtonType,
        mouseButton: MouseButton,
        locationInWindow: LogicalPoint,
        window: Window,
        xdgDesktopSettings: XdgDesktopSettings,
        windowState: WindowState,
    ): EventHandlerResult {
        Logger.info { "executeWindowAction: $mouseButton , $windowButton" }
        return when (windowButton) {
            WindowButtonType.AppMenu, WindowButtonType.Icon -> {
                window.showMenu(locationInWindow)
                EventHandlerResult.Stop
            }
            WindowButtonType.Spacer,
            WindowButtonType.Title,
            -> when (mouseButton) {
                MouseButton.RIGHT -> {
                    executeTitlebarAction(xdgDesktopSettings.actionRightClickTitlebar, window, locationInWindow, windowState)
                    EventHandlerResult.Stop
                }
                MouseButton.MIDDLE -> {
                    executeTitlebarAction(xdgDesktopSettings.actionMiddleClickTitlebar, window, locationInWindow, windowState)
                    EventHandlerResult.Stop
                }
                else -> EventHandlerResult.Continue
            }
            WindowButtonType.Minimize -> {
                window.minimize()
                EventHandlerResult.Stop
            }
            WindowButtonType.Maximize -> {
                toggleMaximize(window, windowState)
                EventHandlerResult.Stop
            }
            WindowButtonType.Close -> {
                requestClose()
                EventHandlerResult.Stop
            }
        }
    }

    private fun handlePotentialDoubleClick(timestamp: Timestamp, doubleClickInterval: Duration): Boolean {
        val prevTime = lastHeaderMouseDownTime
        if (prevTime != null) {
            val timeDiff = (timestamp.toDuration() - prevTime.toDuration())
            Logger.info { "timeDiff: $timeDiff" }
            if (timeDiff <= doubleClickInterval) {
                lastHeaderMouseDownTime = timestamp
                return true
            }
        }
        lastHeaderMouseDownTime = timestamp
        return false
    }

    fun onMouseDown(event: Event.MouseDown): EventHandlerResult {
        val headerRect = LogicalRect(origin, size)
        return if (headerRect.contains(event.locationInWindow) && event.button == MouseButton.LEFT) {
            leftClickStartLocation = event.locationInWindow
            isDragging = false
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }

    fun onMouseUp(
        event: Event.MouseUp,
        xdgDesktopSettings: XdgDesktopSettings,
        window: Window,
        windowState: WindowState,
    ): EventHandlerResult {
        val headerRect = LogicalRect(origin, size)
        val leftClickStartWindowButton = leftClickStartLocation?.let { leftClickStartLocation ->
            rectangles.firstOrNull { it.first.contains(leftClickStartLocation) }?.second
        }
        if (event.button == MouseButton.LEFT) {
            leftClickStartLocation = null
            isDragging = false
        }
        return if (headerRect.contains(event.locationInWindow)) {
            rectangles.firstOrNull { it.first.contains(event.locationInWindow) }?.second?.let { windowButton ->
                if (event.button == MouseButton.LEFT && leftClickStartWindowButton != windowButton) {
                    EventHandlerResult.Continue
                } else if ((windowButton == WindowButtonType.Title || windowButton == WindowButtonType.Spacer) &&
                    event.button == MouseButton.LEFT &&
                    handlePotentialDoubleClick(event.timestamp, xdgDesktopSettings.doubleClickInterval)
                ) {
                    executeTitlebarAction(
                        xdgDesktopSettings.actionDoubleClickTitlebar,
                        window,
                        event.locationInWindow,
                        windowState,
                    )
                    EventHandlerResult.Stop
                } else if (windowButton == WindowButtonType.Minimize && event.button == MouseButton.RIGHT) {
                    window.requestInternalActivationToken()
                    EventHandlerResult.Stop
                } else {
                    executeWindowAction(
                        windowButton,
                        event.button,
                        event.locationInWindow,
                        window,
                        xdgDesktopSettings,
                        windowState,
                    )
                }
            } ?: EventHandlerResult.Continue
        } else {
            EventHandlerResult.Continue
        }
    }

    fun onMouseMoved(locationInWindow: LogicalPoint, window: Window): EventHandlerResult {
        val headerRect = LogicalRect(origin, size)
        lastMouseLocation = locationInWindow
        return if (headerRect.contains(locationInWindow) &&
            !isDragging &&
            (leftClickStartLocation?.isInsideCircle(locationInWindow, MOVE_RADIUS) == false)
        ) {
            isDragging = true
            leftClickStartLocation = null
            window.startMove()
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }

    fun onMouseEntered(): EventHandlerResult {
        isDragging = false
        return EventHandlerResult.Continue
    }

    fun onMouseExited(): EventHandlerResult {
        lastMouseLocation = null
        leftClickStartLocation = null
        return EventHandlerResult.Continue
    }

    private fun drawUnfilledRect(r: Rect, canvas: Canvas, paint: Paint) {
        canvas.drawLine(r.left, r.top, r.left, r.bottom, paint)
        canvas.drawLine(r.left, r.top, r.right, r.top, paint)
        canvas.drawLine(r.right, r.top, r.right, r.bottom, paint)
        canvas.drawLine(r.left, r.bottom, r.right, r.bottom, paint)
    }

    private fun drawButton(
        canvas: Canvas,
        button: WindowButtonType,
        rect: LogicalRect,
        highlighted: Boolean,
        hovered: Boolean,
        scale: Float,
        title: String,
        windowState: WindowState,
    ) {
        val w = rect.size.width * scale
        val h = rect.size.height * scale
        val xOffset = rect.point.x * scale
        val yOffset = rect.point.y * scale

        when (button) {
            WindowButtonType.Minimize, WindowButtonType.Maximize, WindowButtonType.Close, WindowButtonType.AppMenu -> {
                Paint().use { paint ->
                    paint.color = if (highlighted) {
                        COLOR_LIGHT_GRAY
                    } else if (hovered) {
                        COLOR_DARK_GRAY
                    } else {
                        Color.BLACK
                    }
                    canvas.drawRect(Rect.makeXYWH(xOffset, yOffset, w, h), paint)
                }
            }
            else -> {}
        }

        Paint().use { paint ->
            paint.color = Color.WHITE
            paint.strokeWidth = BUTTON_LINE_WIDTH * scale

            val yTop = yOffset + (paint.strokeWidth / 2)
            val yBottom = (yOffset + h) - (paint.strokeWidth / 2)
            val xLeft = xOffset + (paint.strokeWidth / 2) + 1
            val xRight = (xOffset + w) - (paint.strokeWidth / 2) - 2
            when (button) {
                WindowButtonType.AppMenu -> {
                    canvas.drawLine(xLeft, yTop, xRight, yTop, paint)
                    canvas.drawLine(xLeft, yOffset + (h / 2), xRight, yOffset + (h / 2), paint)
                    canvas.drawLine(xLeft, yBottom, xRight, yBottom, paint)
                }
                WindowButtonType.Icon -> {
                    canvas.drawImageRect(
                        APP_ICON,
                        src = APP_ICON.imageInfo.bounds.toRect(),
                        dst = Rect(xLeft, yTop, xRight, yBottom),
                        paint,
                    )
                }
                WindowButtonType.Spacer -> {}
                WindowButtonType.Minimize -> {
                    canvas.drawLine(xLeft, yBottom, xRight, yBottom, paint)
                }
                WindowButtonType.Maximize -> {
                    if (windowState.maximized) {
                        drawUnfilledRect(Rect(xLeft + (w / 5), yTop, xRight, yBottom - (h / 5)), canvas, paint)
                        drawUnfilledRect(Rect(xLeft, yTop + (h / 5), xRight - (w / 5), yBottom), canvas, paint)
                    } else {
                        drawUnfilledRect(Rect(xLeft, yTop, xRight, yBottom), canvas, paint)
                    }
                }
                WindowButtonType.Close -> {
                    canvas.drawLine(xLeft, yTop, xRight, yBottom, paint)
                    canvas.drawLine(xRight, yTop, xLeft, yBottom, paint)
                }
                WindowButtonType.Title -> {
                    paint.color = if (windowState.active) Color.WHITE else COLOR_LIGHT_GRAY
                    canvas.drawTextLine(titleTextLineCreator.makeTextLine(title, CUSTOM_TITLEBAR_HEIGHT * scale), xOffset, yBottom, paint)
                }
            }
        }
    }

    fun draw(canvas: Canvas, scale: Float, xdgDesktopSettings: XdgDesktopSettings, title: String, windowState: WindowState) {
        val physicalOrigin = origin.toPhysical(scale)
        val physicalSize = size.toPhysical(scale)
        val l = physicalOrigin.x.toFloat()
        val t = physicalOrigin.y.toFloat()
        val w = physicalSize.width.toFloat()
        val h = physicalSize.height.toFloat()
        Paint().use { paint ->
            paint.color = xdgDesktopSettings.accentColor
            canvas.drawRect(Rect.makeXYWH(l, t, w, h), paint)
        }
        for ((rect, button) in rectangles) {
            val hovered = !isDragging && (lastMouseLocation?.let { rect.contains(it) } == true)
            val highlighted = hovered && (leftClickStartLocation?.let { rect.contains(it) } == true)
            drawButton(canvas, button, rect, highlighted = highlighted, hovered = hovered, scale, title, windowState)
        }
    }
}
