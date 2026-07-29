package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.DesktopTitlebarAction
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.MouseButton
import org.jetbrains.desktop.linux.Scale
import org.jetbrains.desktop.linux.Timestamp
import org.jetbrains.desktop.linux.Window
import org.jetbrains.desktop.linux.WindowCapabilities
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.Image
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import kotlin.math.pow
import kotlin.time.Duration

private fun jbIconBytes(): ByteArray {
    return object {}.javaClass.getResource("/jb-logo.png")!!.readBytes()
}

private fun LogicalPoint.isInsideCircle(center: LogicalPoint, radius: LogicalPixels): Boolean {
    val xDiff = this.x - center.x
    val yDiff = this.y - center.y
    return xDiff.rawLogical.pow(2) + yDiff.rawLogical.pow(2) <= radius.rawLogical.pow(2)
}

internal class SkikoCustomTitlebarLinux(
    private var headerRect: LogicalDoubleRect,
    private val requestClose: () -> Unit,
) {
    private var buttonLayout = DEFAULT_BUTTON_LAYOUT
    private var rectangles = ArrayList<Pair<LogicalDoubleRect, WindowButtonType>>()
    private var lastHeaderMouseDownTime: Timestamp? = null
    private var lastMouseLocation: LogicalPoint? = null
    private var leftClickStartLocation: LogicalPoint? = null
    private var isDragging: Boolean = false
    private var isActive: Boolean = false
    private var isMaximized: Boolean = false

    private var titleTextLineCreator = TextLineCreator()

    companion object {
        private val BUTTON_LINE_WIDTH = LogicalPixels(5.0)
        private val MOVE_RADIUS = LogicalPixels(3.0)
        private val COLOR_DARK_GRAY = Color.makeRGB(128, 128, 128)
        private val COLOR_LIGHT_GRAY = Color.makeRGB(211, 211, 211)

        private val APP_ICON = Image.makeFromEncoded(jbIconBytes())
        private val DEFAULT_BUTTON_LAYOUT = TitlebarLayout(
            layoutLeft = listOf(WindowButtonType.Icon),
            layoutRight = listOf(WindowButtonType.Minimize, WindowButtonType.Maximize, WindowButtonType.Close),
        )

        private enum class WindowButtonType {
            AppMenu,
            Icon,
            Spacer,
            Title,
            Minimize,
            Maximize,
            Close,
            ;

            companion object {
                fun fromString(buttonName: String): WindowButtonType {
                    return when (buttonName) {
                        "appmenu", "menu" -> AppMenu
                        "icon" -> Icon
                        "spacer" -> Spacer
                        "minimize" -> Minimize
                        "maximize" -> Maximize
                        "close" -> Close
                        else -> error("Unknown button name $buttonName")
                    }
                }
            }
        }

        private data class TitlebarLayout(
            val layoutLeft: List<WindowButtonType>,
            val layoutRight: List<WindowButtonType>,
        ) {
            companion object {
                private fun parseOneSide(buttons: String): List<WindowButtonType> {
                    return if (buttons.isEmpty()) {
                        emptyList()
                    } else {
                        buttons.split(',').map(WindowButtonType::fromString)
                    }
                }

                private fun filterUnsupportedButtons(
                    buttons: List<WindowButtonType>,
                    capabilities: WindowCapabilities?,
                ): List<WindowButtonType> {
                    return buttons
                        .filter {
                            when (it) {
                                WindowButtonType.AppMenu,
                                WindowButtonType.Icon,
                                WindowButtonType.Spacer,
                                WindowButtonType.Title,
                                WindowButtonType.Close,
                                -> true

                                WindowButtonType.Minimize -> capabilities?.minimize ?: true
                                WindowButtonType.Maximize -> capabilities?.maximize ?: true
                            }
                        }
                }

                fun fromString(buttonLayout: String, capabilities: WindowCapabilities?): TitlebarLayout {
                    val (buttonsLeftStr, buttonsRightStr) = buttonLayout.split(':')
                    return TitlebarLayout(
                        filterUnsupportedButtons(parseOneSide(buttonsLeftStr), capabilities),
                        filterUnsupportedButtons(parseOneSide(buttonsRightStr), capabilities),
                    )
                }
            }
        }
    }

    fun setLayout(capabilities: WindowCapabilities?, layoutString: String?) {
        buttonLayout = layoutString?.let {
            TitlebarLayout.fromString(it, capabilities)
        } ?: DEFAULT_BUTTON_LAYOUT

        update()
    }

    fun configure(event: Event.WindowConfigure, rect: LogicalDoubleRect, layoutString: String?) {
        headerRect = rect
        isActive = event.active
        isMaximized = event.maximized
        setLayout(event.capabilities, layoutString)
    }

    fun toggleMaximize(window: Window) {
        if (isMaximized) {
            window.unmaximize()
        } else {
            window.maximize()
        }
    }

    private fun update() {
        val buttonWidth = headerRect.height
        val buttonHeight = headerRect.height
        rectangles.clear()
        buttonLayout.let {
            val buttonsLeftWidth = buttonWidth * it.layoutLeft.size
            val buttonsRightWidth = buttonWidth * it.layoutRight.size
            val titleRect = LogicalDoubleRect(
                x = headerRect.x + buttonsLeftWidth,
                y = headerRect.y,
                width = headerRect.width - buttonsRightWidth - buttonsLeftWidth,
                height = headerRect.height,
            )
            rectangles.add(Pair(titleRect, WindowButtonType.Title))
            for ((i, button) in it.layoutLeft.withIndex()) {
                val rect = LogicalDoubleRect(
                    x = headerRect.x + (buttonWidth * i),
                    y = headerRect.y,
                    width = buttonWidth,
                    height = buttonHeight,
                )
                rectangles.add(Pair(rect, button))
            }
            for ((i, button) in it.layoutRight.withIndex()) {
                val rect = LogicalDoubleRect(
                    x = headerRect.x + headerRect.width - (buttonWidth * (it.layoutRight.size - i)),
                    y = headerRect.y,
                    width = buttonWidth,
                    height = buttonHeight,
                )
                rectangles.add(Pair(rect, button))
            }
        }
    }

    private fun executeTitlebarAction(action: DesktopTitlebarAction, window: Window, event: Event.MouseUp) {
        when (action) {
            DesktopTitlebarAction.ToggleMaximize -> {
                toggleMaximize(window)
            }

            DesktopTitlebarAction.Minimize -> {
                window.minimize()
            }

            DesktopTitlebarAction.None -> {}
            DesktopTitlebarAction.Menu -> {
                window.showMenu(event.locationInWindow)
            }
        }
    }

    private fun executeWindowAction(
        windowButton: WindowButtonType,
        event: Event.MouseUp,
        window: Window,
        xdgDesktopSettings: XdgDesktopSettings,
    ): EventHandlerResult {
        Logger.info { "executeWindowAction: ${event.button} , $windowButton" }
        return when (windowButton) {
            WindowButtonType.AppMenu, WindowButtonType.Icon -> {
                window.showMenu(event.locationInWindow)
                EventHandlerResult.Stop
            }

            WindowButtonType.Spacer,
            WindowButtonType.Title,
            -> when (event.button) {
                MouseButton.RIGHT -> {
                    executeTitlebarAction(xdgDesktopSettings.actionRightClickTitlebar, window, event)
                    EventHandlerResult.Stop
                }

                MouseButton.MIDDLE -> {
                    executeTitlebarAction(xdgDesktopSettings.actionMiddleClickTitlebar, window, event)
                    EventHandlerResult.Stop
                }

                else -> EventHandlerResult.Continue
            }

            WindowButtonType.Minimize -> {
                window.minimize()
                EventHandlerResult.Stop
            }

            WindowButtonType.Maximize -> {
                toggleMaximize(window)
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
        return if (headerRect.contains(event.locationInWindow) && event.button == MouseButton.LEFT) {
            leftClickStartLocation = event.locationInWindow
            isDragging = false
            EventHandlerResult.Stop
        } else {
            EventHandlerResult.Continue
        }
    }

    fun onMouseUp(event: Event.MouseUp, xdgDesktopSettings: XdgDesktopSettings, window: Window): EventHandlerResult {
        val leftClickStartWindowButton = leftClickStartLocation?.let { leftClickStartLocation ->
            rectangles.firstOrNull { it.first.contains(leftClickStartLocation) }?.second
        }
        if (event.button == MouseButton.LEFT) {
            leftClickStartLocation = null
            isDragging = false
        }
        return rectangles.firstOrNull { it.first.contains(event.locationInWindow) }?.second?.let { windowButton ->
            if (event.button == MouseButton.LEFT && leftClickStartWindowButton != windowButton) {
                EventHandlerResult.Continue
            } else if ((windowButton == WindowButtonType.Title || windowButton == WindowButtonType.Spacer) &&
                event.button == MouseButton.LEFT &&
                handlePotentialDoubleClick(event.timestamp, xdgDesktopSettings.doubleClickInterval)
            ) {
                executeTitlebarAction(
                    xdgDesktopSettings.actionDoubleClickTitlebar,
                    window,
                    event,
                )
                EventHandlerResult.Stop
            } else if (windowButton == WindowButtonType.Minimize && event.button == MouseButton.RIGHT) {
                window.requestInternalActivationToken()
                EventHandlerResult.Stop
            } else {
                executeWindowAction(
                    windowButton,
                    event,
                    window,
                    xdgDesktopSettings,
                )
            }
        } ?: EventHandlerResult.Continue
    }

    fun onMouseMoved(locationInWindow: LogicalPoint, window: Window): EventHandlerResult {
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
        rect: Rect,
        highlighted: Boolean,
        hovered: Boolean,
        scale: Scale,
        title: String,
    ) {
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
                    canvas.drawRect(rect, paint)
                }
            }

            else -> {}
        }

        Paint().use { paint ->
            paint.color = Color.WHITE
            paint.strokeWidth = BUTTON_LINE_WIDTH.toSkiko(scale)

            val yTop = rect.top + (paint.strokeWidth / 2)
            val yBottom = (rect.top + rect.height) - (paint.strokeWidth / 2)
            val xLeft = rect.left + (paint.strokeWidth / 2) + 1
            val xRight = (rect.left + rect.width) - (paint.strokeWidth / 2) - 2
            when (button) {
                WindowButtonType.AppMenu -> {
                    canvas.drawLine(xLeft, yTop, xRight, yTop, paint)
                    canvas.drawLine(xLeft, rect.top + (rect.height / 2), xRight, rect.top + (rect.height / 2), paint)
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
                    if (isMaximized) {
                        drawUnfilledRect(Rect(xLeft + (rect.width / 5), yTop, xRight, yBottom - (rect.height / 5)), canvas, paint)
                        drawUnfilledRect(Rect(xLeft, yTop + (rect.height / 5), xRight - (rect.width / 5), yBottom), canvas, paint)
                    } else {
                        drawUnfilledRect(Rect(xLeft, yTop, xRight, yBottom), canvas, paint)
                    }
                }

                WindowButtonType.Close -> {
                    canvas.drawLine(xLeft, yTop, xRight, yBottom, paint)
                    canvas.drawLine(xRight, yTop, xLeft, yBottom, paint)
                }

                WindowButtonType.Title -> {
                    val color = if (isActive) Color.WHITE else COLOR_LIGHT_GRAY
                    titleTextLineCreator.makeTextLine(
                        title,
                        (headerRect.height * 2 / 3).toSkiko(scale),
                        color,
                        rect.width,
                    ).paint(canvas, xLeft, yTop)
                }
            }
        }
    }

    fun draw(canvas: Canvas, xdgDesktopSettings: XdgDesktopSettings, title: String, scale: Scale) {
        Paint().use { paint ->
            paint.color = xdgDesktopSettings.accentColor
            canvas.drawRect(headerRect.toSkiko(scale), paint)
        }
        for ((rect, button) in rectangles) {
            val hovered = !isDragging && (lastMouseLocation?.let { rect.contains(it) } == true)
            val highlighted = hovered && (leftClickStartLocation?.let { rect.contains(it) } == true)
            drawButton(canvas, button, rect.toSkiko(scale), highlighted = highlighted, hovered = hovered, scale, title)
        }
    }
}
