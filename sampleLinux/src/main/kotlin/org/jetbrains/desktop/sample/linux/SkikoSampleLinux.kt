package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.ApplicationConfig
import org.jetbrains.desktop.linux.ColorSchemeValue
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.FontAntialiasingValue
import org.jetbrains.desktop.linux.FontHintingValue
import org.jetbrains.desktop.linux.FontRgbaOrderValue
import org.jetbrains.desktop.linux.KotlinDesktopToolkit
import org.jetbrains.desktop.linux.LogLevel
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalRect
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.MouseButton
import org.jetbrains.desktop.linux.PhysicalPoint
import org.jetbrains.desktop.linux.PhysicalSize
import org.jetbrains.desktop.linux.PointerShape
import org.jetbrains.desktop.linux.Timestamp
import org.jetbrains.desktop.linux.WindowButtonType
import org.jetbrains.desktop.linux.WindowCapabilities
import org.jetbrains.desktop.linux.WindowFrameAction
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.desktop.linux.WindowResizeEdge
import org.jetbrains.desktop.linux.XdgDesktopSetting
import org.jetbrains.desktop.linux.XdgDesktopSetting.TitlebarLayout
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.Font
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.FontStyle
import org.jetbrains.skia.Image
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import org.jetbrains.skia.TextLine
import java.lang.AutoCloseable
import java.nio.file.Files
import java.nio.file.Path
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin
import kotlin.time.Duration
import kotlin.time.DurationUnit
import kotlin.time.toDuration

data class XdgDesktopSettings(
    var titlebarLayout: TitlebarLayout = TitlebarLayout(
        layoutLeft = listOf(WindowButtonType.Icon),
        layoutRight = listOf(WindowButtonType.Minimize, WindowButtonType.Maximize, WindowButtonType.Close),
    ),
    var doubleClickIntervalMs: Int = 500,
    var colorScheme: ColorSchemeValue = ColorSchemeValue.NoPreference,
    var accentColor: Int = Color.BLUE,
    var fontAntialiasing: FontAntialiasingValue = FontAntialiasingValue.Grayscale,
    var fontHinting: FontHintingValue = FontHintingValue.Medium,
    var fontRgbaOrder: FontRgbaOrderValue = FontRgbaOrderValue.Rgb,
    var cursorBlink: Boolean = true,
    var cursorBlinkTime: Duration = 1200.toDuration(DurationUnit.MILLISECONDS),
    var cursorBlinkTimeout: Duration = 10.toDuration(DurationUnit.SECONDS),
    var overlayScrolling: Boolean = false,
    var audibleBell: Boolean = true,
) {
    private fun colorDoubleToInt(v: Double): Int = (v * 255).roundToInt()

    fun update(s: XdgDesktopSetting) {
        when (s) {
            is TitlebarLayout -> titlebarLayout = s
            is XdgDesktopSetting.DoubleClickInterval -> doubleClickIntervalMs = s.intervalMs
            is XdgDesktopSetting.ColorScheme -> colorScheme = s.value
            is XdgDesktopSetting.AccentColor -> accentColor = Color.makeARGB(
                a = colorDoubleToInt(s.value.alpha),
                r = colorDoubleToInt(s.value.red),
                g = colorDoubleToInt(s.value.green),
                b = colorDoubleToInt(s.value.blue),
            )
            is XdgDesktopSetting.FontAntialiasing -> fontAntialiasing = s.value
            is XdgDesktopSetting.FontHinting -> fontHinting = s.value
            is XdgDesktopSetting.FontRgbaOrder -> fontRgbaOrder = s.value
            is XdgDesktopSetting.AudibleBell -> audibleBell = s.value
            is XdgDesktopSetting.CursorBlink -> cursorBlink = s.value
            is XdgDesktopSetting.CursorBlinkTime -> cursorBlinkTime = s.value
            is XdgDesktopSetting.CursorBlinkTimeout -> cursorBlinkTimeout = s.value
            is XdgDesktopSetting.OverlayScrolling -> overlayScrolling = s.value
        }
    }
}

class CustomTitlebar(
    private var origin: LogicalPoint,
    var size: LogicalSize,
    var buttonLayout: TitlebarLayout,
) {
    private var rectangles = ArrayList<Pair<LogicalRect, WindowButtonType>>()
    private var mouseOverRectIndex: Int? = null
    private var maximized: Boolean = false
    private var fullscreen: Boolean = false
    private var lastHeaderMouseDownTime: Timestamp? = null

    private data class TitleParams(private var fontSize: Float, private var title: String) {
        private var titleLine: TextLine? = null

        fun makeTitleLine(title: String, fontSize: Float): TextLine {
            if (titleLine == null || this.title != title || this.fontSize != fontSize) {
                this.title = title
                this.fontSize = fontSize
                val titlebarFont = FontMgr.default.matchFamilyStyle("sans-serif", FontStyle.BOLD)?.let { typeface ->
                    Font(typeface, this.fontSize)
                }
                titleLine = TextLine.make(this.title, titlebarFont)
            }
            return titleLine!!
        }
    }

    private var lastTitleParams = TitleParams(fontSize = 0f, title = "")

    companion object {
        const val CUSTOM_TITLEBAR_HEIGHT: LogicalPixels = 55f
        const val BUTTON_LINE_WIDTH: LogicalPixels = 5f
        val BUTTON_SIZE = LogicalSize(CUSTOM_TITLEBAR_HEIGHT, CUSTOM_TITLEBAR_HEIGHT)

        val APP_ICON = Image.makeFromEncoded(Files.readAllBytes(Path.of("resources/jb-logo.png")))
    }

    fun resize(event: Event.WindowResize, layout: TitlebarLayout) {
        size = LogicalSize(width = event.size.width, height = CUSTOM_TITLEBAR_HEIGHT)
        maximized = event.maximized
        fullscreen = event.fullscreen
        mouseOverRectIndex = null
        setLayout(layout)
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

    private fun toMouseClickAction(
        windowButton: WindowButtonType,
        mouseButton: MouseButton,
        locationInWindow: LogicalPoint,
    ): WindowFrameAction? {
        return when (windowButton) {
            WindowButtonType.AppMenu -> WindowFrameAction.ShowMenu(locationInWindow)
            WindowButtonType.Icon -> WindowFrameAction.ShowMenu(locationInWindow)
            WindowButtonType.Spacer,
            WindowButtonType.Title,
            -> when (mouseButton) {
                MouseButton.LEFT -> WindowFrameAction.Move
                MouseButton.RIGHT -> WindowFrameAction.ShowMenu(locationInWindow)
                else -> null
            }
            WindowButtonType.Minimize -> WindowFrameAction.Minimize
            WindowButtonType.Maximize -> if (maximized) WindowFrameAction.UnMaximize else WindowFrameAction.Maximize
            WindowButtonType.Close -> WindowFrameAction.Close
        }
    }

    fun handleEvent(event: Event, xdgDesktopSettings: XdgDesktopSettings): EventHandlerResult {
        when (event) {
            is Event.MouseDown -> {
                for ((rect, windowButton) in rectangles) {
                    if (rect.contains(event.locationInWindow)) {
                        if (windowButton == WindowButtonType.Title || windowButton == WindowButtonType.Spacer) {
                            val prevTime = lastHeaderMouseDownTime
                            if (prevTime != null) {
                                val timeDiff = (event.timestamp.toDuration() - prevTime.toDuration()).inWholeMilliseconds
                                Logger.info { "timeDiff: $timeDiff" }
                                if (timeDiff <= xdgDesktopSettings.doubleClickIntervalMs) {
                                    event.setFrameAction(if (maximized) WindowFrameAction.UnMaximize else WindowFrameAction.Maximize)
                                    lastHeaderMouseDownTime = event.timestamp
                                    return EventHandlerResult.Stop
                                }
                            }
                            lastHeaderMouseDownTime = event.timestamp
                        }
                        toMouseClickAction(windowButton, event.button, event.locationInWindow)?.let {
                            event.setFrameAction(it)
                            return EventHandlerResult.Stop
                        }
                        break
                    }
                }
            }
            is Event.MouseMoved -> {
                mouseOverRectIndex = null
                for ((i, v) in rectangles.withIndex()) {
                    val rect = v.first
                    if (rect.contains(event.locationInWindow)) {
                        mouseOverRectIndex = i
                        break
                    }
                }
            }
            else -> {}
        }
        return EventHandlerResult.Continue
    }

    private fun drawButton(canvas: Canvas, button: WindowButtonType, rect: LogicalRect, highlighted: Boolean, scale: Float, title: String) {
        val w = rect.size.width * scale
        val h = rect.size.height * scale
        val xOffset = rect.point.x * scale
        val yOffset = rect.point.y * scale

        when (button) {
            WindowButtonType.Minimize, WindowButtonType.Maximize, WindowButtonType.Close -> {
                Paint().use { paint ->
                    paint.color = if (highlighted) Color.WHITE else 0xFFD3D3D3.toInt()
                    canvas.drawRect(Rect.makeXYWH(xOffset, yOffset, w, h), paint)
                }
            }
            else -> {}
        }

        Paint().use { paint ->
            paint.color = Color.BLACK
            paint.strokeWidth = BUTTON_LINE_WIDTH * scale

            val yTop = yOffset + (paint.strokeWidth / 2)
            val yBottom = (yOffset + h) - (paint.strokeWidth / 2)
            val xLeft = xOffset + (paint.strokeWidth / 2)
            val xRight = (xOffset + w) - (paint.strokeWidth / 2)
            when (button) {
                WindowButtonType.AppMenu -> {
                    canvas.drawLine(xOffset, yOffset, xOffset + w, yOffset, paint)
                    canvas.drawLine(xOffset, yOffset + (h / 2), xOffset + w, yOffset + (h / 2), paint)
                    canvas.drawLine(xOffset, yBottom, xOffset + w, yBottom, paint)
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
                    canvas.drawLine(xOffset, yBottom, xOffset + w, yBottom, paint)
                }
                WindowButtonType.Maximize -> {
                    canvas.drawLine(xLeft, yTop, xLeft, yBottom, paint)
                    canvas.drawLine(xLeft, yTop, xRight, yTop, paint)
                    canvas.drawLine(xRight, yTop, xRight, yBottom, paint)
                    canvas.drawLine(xLeft, yBottom, xRight, yBottom, paint)
                }
                WindowButtonType.Close -> {
                    canvas.drawLine(xOffset, yOffset, xOffset + w, yBottom, paint)
                    canvas.drawLine(xOffset + w, yOffset, xOffset, yBottom, paint)
                }
                WindowButtonType.Title -> {
                    paint.color = Color.WHITE
                    canvas.drawTextLine(lastTitleParams.makeTitleLine(title, CUSTOM_TITLEBAR_HEIGHT * scale), xOffset, yBottom, paint)
                }
            }
        }
    }

    fun draw(canvas: Canvas, scale: Float, xdgDesktopSettings: XdgDesktopSettings, title: String) {
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
        for ((i, v) in rectangles.withIndex()) {
            drawButton(canvas, v.second, v.first, highlighted = mouseOverRectIndex == i, scale, title)
        }
    }
}

class ContentArea(
    var origin: LogicalPoint,
    var size: LogicalSize,
) {
    private var markerPosition: LogicalPoint? = null

    fun handleEvent(event: Event): EventHandlerResult {
        if (event is Event.MouseMoved) {
            markerPosition = LogicalPoint(
                event.locationInWindow.x - origin.x,
                event.locationInWindow.y - origin.y,
            )
        }
        return EventHandlerResult.Continue
    }

    fun draw(canvas: Canvas, time: Long, scale: Float) {
        val contentOrigin = origin.toPhysical(scale)
        val contentSize = size.toPhysical(scale)
        Paint().use { paint ->
            paint.color = 0x77264653
            canvas.drawRect(
                Rect.makeXYWH(
                    contentOrigin.x.toFloat(),
                    contentOrigin.y.toFloat(),
                    contentSize.width.toFloat(),
                    contentSize.height.toFloat(),
                ),
                paint,
            )
        }
        canvas.withTranslated(contentOrigin) {
            Paint().use { paint ->
                paint.color = Color.WHITE
                paint.strokeWidth = scale
                canvas.drawLine(
                    contentSize.width.toFloat(),
                    0f,
                    0f,
                    contentSize.height.toFloat(),
                    paint,
                )
            }
        }
        canvas.drawSpiningCircle(contentOrigin, contentSize, time)
        canvas.drawWindowBorders(contentOrigin, contentSize, scale)
        canvas.drawCursor(contentOrigin, contentSize, scale)
    }

    private fun Canvas.drawSpiningCircle(origin: PhysicalPoint, size: PhysicalSize, t: Long) = withTranslated(origin) {
        val width = size.width.toFloat()
        val height = size.height.toFloat()
        val angle = (t / 2000f) * 2f * PI
        val r = width / 4
        val x = r * sin(angle).toFloat() + width / 2f
        val y = r * cos(angle).toFloat() + height / 2f
        Paint().use { paint ->
            paint.color = Color.GREEN
            drawCircle(x, y, 30f, paint)
        }
    }

    private fun Canvas.drawWindowBorders(origin: PhysicalPoint, size: PhysicalSize, scale: Float) = withTranslated(origin) {
        val width = size.width.toFloat()
        val height = size.height.toFloat()
        Paint().use { paint ->
            val barSize = 3 * scale

            // left
            paint.color = 0xFFe76f51.toInt()
            drawRect(Rect.makeXYWH(0f, 0f, barSize, 100 * scale), paint)
            drawRect(Rect.makeXYWH(0f, height / 2 - 50 * scale, barSize, 100 * scale), paint)
            drawRect(Rect.makeXYWH(0f, height - 100 * scale, barSize, 100 * scale), paint)

            // top
            paint.color = 0xFF2a9d8f.toInt()
            drawRect(Rect.makeXYWH(0f, 0f, 100 * scale, barSize), paint)
            drawRect(Rect.makeXYWH(width / 2 - 50 * scale, 0f, 100 * scale, barSize), paint)
            drawRect(Rect.makeXYWH(width - 100 * scale, 0f, 100 * scale, barSize), paint)

            // right
            paint.color = 0xFFe9c46a.toInt()
            drawRect(Rect.makeXYWH(width - barSize, 0f, barSize, 100 * scale), paint)
            drawRect(Rect.makeXYWH(width - barSize, height / 2 - 50 * scale, barSize, 100 * scale), paint)
            drawRect(Rect.makeXYWH(width - barSize, height - 100 * scale, barSize, 100 * scale), paint)

            // bottom
            paint.color = 0xFFFFFFFF.toInt()
            drawRect(Rect.makeXYWH(0f, height - barSize, 100 * scale, barSize), paint)
            drawRect(Rect.makeXYWH(width / 2 - 50 * scale, height - barSize, 100 * scale, barSize), paint)
            drawRect(Rect.makeXYWH(width - 100 * scale, height - barSize, 100 * scale, barSize), paint)
        }
    }

    private fun Canvas.drawCursor(origin: PhysicalPoint, size: PhysicalSize, scale: Float) = withTranslated(origin) {
        val canvas = this
        markerPosition?.let { curs ->
            val positive = curs.x > 0 && curs.y > 0
            val inBox = curs.x < size.width && curs.y < size.height
            if (positive && inBox) {
                val x = curs.x
                val y = curs.y
                val width = size.width.toFloat()
                val height = size.height.toFloat()

                Paint().use { paint ->
                    paint.color = 0x40FFFFFF
                    canvas.drawRect(Rect.makeXYWH(0f, y * scale, width, 2 * scale), paint)
                    canvas.drawRect(Rect.makeXYWH(x * scale, 0f, 2 * scale, height), paint)
                }
            }
        }
    }
}

class CustomBorders {
    companion object {
        const val BORDER_SIZE: LogicalPixels = 5f

        fun edgeToPointerShape(edge: WindowResizeEdge): PointerShape {
            return when (edge) {
                WindowResizeEdge.Top -> PointerShape.NResize
                WindowResizeEdge.Bottom -> PointerShape.SResize
                WindowResizeEdge.Left -> PointerShape.WResize
                WindowResizeEdge.TopLeft -> PointerShape.NwResize
                WindowResizeEdge.BottomLeft -> PointerShape.SwResize
                WindowResizeEdge.Right -> PointerShape.EResize
                WindowResizeEdge.TopRight -> PointerShape.NeResize
                WindowResizeEdge.BottomRight -> PointerShape.SeResize
            }
        }
    }

    private var rectangles = ArrayList<Pair<LogicalRect, WindowResizeEdge>>()

    fun resize(event: Event.WindowResize) {
        rectangles.clear()
        val edgeSize = LogicalSize(BORDER_SIZE, BORDER_SIZE)
        rectangles.add(Pair(LogicalRect(LogicalPoint.Zero, edgeSize), WindowResizeEdge.TopLeft))
        rectangles.add(Pair(LogicalRect(LogicalPoint(event.size.width - BORDER_SIZE, 0f), edgeSize), WindowResizeEdge.TopRight))
        rectangles.add(Pair(LogicalRect(LogicalPoint(0f, event.size.height - BORDER_SIZE), edgeSize), WindowResizeEdge.BottomLeft))
        rectangles.add(
            Pair(
                LogicalRect(LogicalPoint(event.size.width - BORDER_SIZE, event.size.height - BORDER_SIZE), edgeSize),
                WindowResizeEdge.BottomRight,
            ),
        )

        rectangles.add(Pair(LogicalRect(LogicalPoint.Zero, LogicalSize(BORDER_SIZE, event.size.height)), WindowResizeEdge.Left))
        rectangles.add(
            Pair(
                LogicalRect(LogicalPoint(event.size.width - BORDER_SIZE, 0f), LogicalSize(BORDER_SIZE, event.size.height)),
                WindowResizeEdge.Right,
            ),
        )
        rectangles.add(Pair(LogicalRect(LogicalPoint.Zero, LogicalSize(event.size.width, BORDER_SIZE)), WindowResizeEdge.Top))
        rectangles.add(
            Pair(
                LogicalRect(LogicalPoint(0f, event.size.height - BORDER_SIZE), LogicalSize(event.size.width, BORDER_SIZE)),
                WindowResizeEdge.Bottom,
            ),
        )
    }

    fun toEdge(locationInWindow: LogicalPoint): WindowResizeEdge? {
        for ((rect, edge) in rectangles) {
            if (rect.contains(locationInWindow)) {
                return edge
            }
        }
        return null
    }

    fun handleEvent(event: Event): EventHandlerResult {
        when (event) {
            is Event.MouseDown -> {
                val edge = toEdge(event.locationInWindow)
                if (edge != null) {
                    event.setFrameAction(WindowFrameAction.Resize(edge))
                    return EventHandlerResult.Stop
                }
            }
            is Event.MouseMoved -> {
            }
            else -> {}
        }
        return EventHandlerResult.Continue
    }
}

class WindowContainer(
    var customTitlebar: CustomTitlebar?,
    var customBorders: CustomBorders?,
    private val contentArea: ContentArea,
    private var xdgDesktopSettings: XdgDesktopSettings,
) {
    private var capabilities: WindowCapabilities? = null

    companion object {
        fun create(windowContentSize: LogicalSize, xdgDesktopSettings: XdgDesktopSettings): WindowContainer {
            val contentArea = ContentArea(LogicalPoint.Zero, windowContentSize)
            return WindowContainer(null, customBorders = null, contentArea, xdgDesktopSettings)
        }

        private fun filterUnsupportedButtons(buttons: List<WindowButtonType>, capabilities: WindowCapabilities): List<WindowButtonType> {
            return buttons
                .filter {
                    when (it) {
                        WindowButtonType.AppMenu,
                        WindowButtonType.Icon,
                        WindowButtonType.Spacer,
                        WindowButtonType.Title,
                        WindowButtonType.Close,
                        -> true
                        WindowButtonType.Minimize -> capabilities.minimize
                        WindowButtonType.Maximize -> capabilities.maximixe
                    }
                }
        }
    }

    fun settingsChanged(xdgDesktopSettings: XdgDesktopSettings) {
        this.xdgDesktopSettings = xdgDesktopSettings
        capabilities?.let { capabilities ->
            customTitlebar?.setLayout(
                TitlebarLayout(
                    layoutLeft = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutLeft, capabilities),
                    layoutRight = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutRight, capabilities),
                ),
            )
        }
    }

    fun resize(event: Event.WindowResize) {
        capabilities = event.capabilities
        if (event.clientSideDecorations) {
            val titlebarLayout = TitlebarLayout(
                layoutLeft = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutLeft, event.capabilities),
                layoutRight = filterUnsupportedButtons(xdgDesktopSettings.titlebarLayout.layoutRight, event.capabilities),
            )
            val titlebarSize = LogicalSize(width = event.size.width, height = CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT)
            val titlebar = customTitlebar ?: CustomTitlebar(origin = LogicalPoint.Zero, size = titlebarSize, titlebarLayout).also {
                customTitlebar = it
            }
            titlebar.resize(event, titlebarLayout)
            val customBorders = customBorders ?: CustomBorders().also { customBorders = it }
            customBorders.resize(event)
            contentArea.origin = LogicalPoint(x = 0f, y = titlebar.size.height)
            contentArea.size =
                LogicalSize(width = event.size.width, height = event.size.height - titlebar.size.height)
        } else {
            contentArea.size = event.size
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
        return when {
            customBorders?.handleEvent(event) == EventHandlerResult.Stop -> EventHandlerResult.Stop
            customTitlebar?.handleEvent(event, xdgDesktopSettings) == EventHandlerResult.Stop -> EventHandlerResult.Stop
            contentArea.handleEvent(event) == EventHandlerResult.Stop -> EventHandlerResult.Stop
            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Float, title: String) {
        val backgroundColor = if (xdgDesktopSettings.colorScheme == ColorSchemeValue.PreferDark) {
            Color.makeARGB(
                240,
                32,
                32,
                32,
            )
        } else {
            Color.makeARGB(240, 200, 200, 200)
        }
        canvas.clear(backgroundColor)
        customTitlebar?.draw(canvas, scale, xdgDesktopSettings, title)
        contentArea.draw(canvas, time, scale)
    }
}

class RotatingBallWindow(
    private val windowContainer: WindowContainer,
    app: Application,
    windowParams: WindowParams,
) : SkikoWindowLinux(app, windowParams) {
    private var title: String = windowParams.title

    companion object {
        fun createWindow(app: Application, windowParams: WindowParams, xdgDesktopSettings: XdgDesktopSettings): RotatingBallWindow {
            val windowSize = LogicalSize(640f, 480f)
            val windowContentSize = windowSize // todo it's incorrect
            val container = WindowContainer.create(windowContentSize, xdgDesktopSettings)

            return RotatingBallWindow(container, app, windowParams)
        }
    }

    fun settingsChanged(xdgDesktopSettings: XdgDesktopSettings) {
        windowContainer.settingsChanged(xdgDesktopSettings)
    }

    override fun handleEvent(event: Event): EventHandlerResult {
        return if (super.handleEvent(event) == EventHandlerResult.Continue) {
            when (event) {
                is Event.WindowResize -> {
                    windowContainer.resize(event)
                    // performDrawing(syncWithCA = true)
                    EventHandlerResult.Stop
                }
                is Event.MouseMoved -> {
                    val borderEdge = windowContainer.customBorders?.toEdge(event.locationInWindow)
                    if (borderEdge != null) {
                        window.setPointerShape(CustomBorders.edgeToPointerShape(borderEdge))
                        EventHandlerResult.Stop
                    } else {
                        window.setPointerShape(PointerShape.Default)
                        windowContainer.handleEvent(event)
                    }
                }
                else -> {
                    windowContainer.handleEvent(event)
                }
            }
        } else {
            EventHandlerResult.Stop
        }
    }

    override fun Canvas.draw(size: PhysicalSize, time: Long) {
        val canvas = this
        windowContainer.draw(canvas, time, window.scaleFactor().toFloat(), title)
    }
}

class ApplicationState(private val app: Application) : AutoCloseable {
    private val windows = mutableListOf<RotatingBallWindow>()
    private var xdgDesktopSettings = XdgDesktopSettings()

    fun createWindow(useCustomTitlebar: Boolean, forceSoftwareRendering: Boolean = false) {
        val windowParams = WindowParams(
            size = LogicalSize(width = 640f, height = 480f),
            title = "Window ${windows.count()}",
            appId = "org.jetbrains.desktop.linux.skikoSample1",
            forceClientSideDecoration = useCustomTitlebar,
            forceSoftwareRendering = forceSoftwareRendering,
        )

        windows.add(
            RotatingBallWindow.createWindow(
                app,
                windowParams,
                xdgDesktopSettings,
            ),
        )
    }

    fun settingChanged(s: XdgDesktopSetting) {
        this.xdgDesktopSettings.update(s)
        windows.forEach { it.settingsChanged(xdgDesktopSettings) }
    }

    override fun close() {
        windows.forEach {
            it.close()
        }
    }
}

fun main(args: Array<String>) {
    if (args.isNotEmpty()) {
        Logger.info { "args = $args" }
    }
    Logger.info { runtimeInfo() }
    KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Debug)
    val app = Application()
    ApplicationState(app).use { state ->
        state.createWindow(useCustomTitlebar = true, forceSoftwareRendering = false)
        app.runEventLoop(ApplicationConfig(onXdgDesktopSettingsChange = { state.settingChanged(it) }))
    }
}
