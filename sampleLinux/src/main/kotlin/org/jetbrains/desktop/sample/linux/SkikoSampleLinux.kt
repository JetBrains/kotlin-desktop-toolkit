package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.ApplicationConfig
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.KotlinDesktopToolkit
import org.jetbrains.desktop.linux.LogLevel
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.LogicalPixels
import org.jetbrains.desktop.linux.LogicalPoint
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.PhysicalPoint
import org.jetbrains.desktop.linux.PhysicalSize
import org.jetbrains.desktop.linux.WindowButtonType
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import java.lang.AutoCloseable
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

class CustomTitlebar(
    private var origin: LogicalPoint,
    var size: LogicalSize,
    var startWindowDrag: (() -> Unit)? = null,
    var buttonLayout: Pair<List<WindowButtonType>, List<WindowButtonType>>? = null,
) {
    companion object {
        const val CUSTOM_TITLEBAR_HEIGHT: LogicalPixels = 55.0
    }

    fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.MouseDown -> {
                if (event.locationInWindow.x > origin.x &&
                    event.locationInWindow.x < origin.x + size.width * 0.75 &&
                    event.locationInWindow.y > origin.y &&
                    event.locationInWindow.y < origin.y + size.height
                ) {
                    startWindowDrag?.invoke()
                    EventHandlerResult.Stop
                } else {
                    EventHandlerResult.Continue
                }
            }
            else -> EventHandlerResult.Continue
        }
    }

    private fun drawButton(canvas: Canvas, button: WindowButtonType, xOffset: Float, scale: Float) {
        val buttonSize = 50f * scale
        val yOffset = 0f * scale
        Paint().use { paint ->
            paint.color = Color.BLACK
            paint.strokeWidth = 5f * scale
            when (button) {
                WindowButtonType.AppMenu -> {
                    canvas.drawLine(xOffset, yOffset, xOffset + buttonSize, yOffset, paint)
                    canvas.drawLine(xOffset, yOffset + (buttonSize / 2), xOffset + buttonSize, yOffset + (buttonSize / 2), paint)
                    canvas.drawLine(xOffset, yOffset + buttonSize, xOffset + buttonSize, yOffset + buttonSize, paint)
                }
                WindowButtonType.Icon -> {
                    // canvas.drawImage()
                }
                WindowButtonType.Spacer -> {}
                WindowButtonType.Minimize -> {
                    canvas.drawLine(xOffset, yOffset + buttonSize, xOffset + buttonSize, yOffset + buttonSize, paint)
                }
                WindowButtonType.Maximize -> {
                    canvas.drawRect(Rect.makeXYWH(xOffset, yOffset, buttonSize, buttonSize), paint)
                }
                WindowButtonType.Close -> {
                    canvas.drawLine(xOffset, yOffset, xOffset + buttonSize, yOffset + buttonSize, paint)
                    canvas.drawLine(xOffset + buttonSize, yOffset, xOffset, yOffset + buttonSize, paint)
                }
            }
        }
    }

    fun draw(canvas: Canvas, scale: Double) {
        val physicalOrigin = origin.toPhysical(scale)
        val physicalSize = size.toPhysical(scale)
        val x = physicalOrigin.x.toFloat()
        val y = physicalOrigin.y.toFloat()
        val width = physicalSize.width.toFloat()
        val height = physicalSize.height.toFloat()
        Paint().use { paint ->
            paint.color = 0xFF404040.toInt()
            canvas.drawRect(Rect.makeXYWH(x, y, width, height), paint)
        }
        buttonLayout?.let { (buttonsLeft, buttonsRight) ->
            for ((i, b) in buttonsLeft.withIndex()) {
                drawButton(canvas, b, i * 60f * scale.toFloat(), scale.toFloat())
            }
            for ((i, b) in buttonsRight.reversed().withIndex()) {
                drawButton(canvas, b, width - (i * 60f * scale.toFloat()), scale.toFloat())
            }
        }
    }
}

class ContentArea(
    var origin: LogicalPoint,
    var size: LogicalSize,
) {

    private var markerPosition: LogicalPoint? = null

    fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.MouseMoved -> {
                markerPosition = LogicalPoint(
                    event.locationInWindow.x - origin.x,
                    event.locationInWindow.y - origin.y,
                )
                EventHandlerResult.Continue
            }

            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Double) {
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
        canvas.drawSpiningCircle(contentOrigin, contentSize, time)
        canvas.drawWindowBorders(contentOrigin, contentSize, scale.toFloat())
        canvas.drawCursor(contentOrigin, contentSize, scale.toFloat())
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
                val x = curs.x.toFloat()
                val y = curs.y.toFloat()
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

class WindowContainer(
    var customTitlebar: CustomTitlebar?,
    private val contentArea: ContentArea,
) {
    companion object {
        fun create(windowContentSize: LogicalSize): WindowContainer {
            val contentArea = ContentArea(LogicalPoint.Zero, windowContentSize)
            return WindowContainer(null, contentArea)
        }
    }

    fun resize(event: Event.WindowResize) {
        if (event.titlebarLayout != null) {
            val titlebarSize = LogicalSize(width = event.size.width, height = CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT)
            val titlebar = customTitlebar ?: CustomTitlebar(origin = LogicalPoint.Zero, size = titlebarSize).also {
                customTitlebar = it
            }
            titlebar.buttonLayout = event.titlebarLayout
            titlebar.size = LogicalSize(width = event.size.width, height = CustomTitlebar.CUSTOM_TITLEBAR_HEIGHT)
            contentArea.origin = LogicalPoint(x = 0.0, y = titlebar.size.height)
            contentArea.size =
                LogicalSize(width = event.size.width, height = event.size.height - titlebar.size.height)
        } else {
            contentArea.size = event.size
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
        return when (EventHandlerResult.Stop) {
            customTitlebar?.handleEvent(event) -> EventHandlerResult.Stop
            contentArea.handleEvent(event) -> EventHandlerResult.Stop
            else -> EventHandlerResult.Continue
        }
    }

    fun draw(canvas: Canvas, time: Long, scale: Double) {
        customTitlebar?.draw(canvas, scale)
        contentArea.draw(canvas, time, scale)
    }
}

class RotatingBallWindow(
    private val windowContainer: WindowContainer,
    app: Application,
    windowParams: WindowParams,
) : SkikoWindowLinux(app, windowParams) {
    companion object {
        fun createWindow(app: Application, title: String, useCustomTitlebar: Boolean): RotatingBallWindow {
            val windowSize = LogicalSize(640.0, 480.0)
            val windowContentSize = windowSize // todo it's incorrect
            val container = WindowContainer.create(windowContentSize)

            val windowParams = WindowParams(
                width = 640,
                height = 480,
                title = title,
                forceClientSideDecoration = useCustomTitlebar,
            )

            return RotatingBallWindow(container, app, windowParams)
        }
    }

    override fun handleEvent(event: Event): EventHandlerResult {
        return if (super.handleEvent(event) == EventHandlerResult.Continue) {
            when {
                event is Event.WindowResize -> {
                    windowContainer.resize(event)
                    // performDrawing(syncWithCA = true)
                    EventHandlerResult.Stop
                }
            }
            windowContainer.customTitlebar?.startWindowDrag = {
//                window.startDrag()
            }
            windowContainer.handleEvent(event)
        } else {
            EventHandlerResult.Stop
        }
    }

    override fun Canvas.draw(size: PhysicalSize, time: Long) {
        val canvas = this
//        canvas.clear(Color.TRANSPARENT) // use RED to debug
        windowContainer.draw(canvas, time, window.scaleFactor())
    }
}

class ApplicationState(private val app: Application) : AutoCloseable {
    private val windows = mutableListOf<RotatingBallWindow>()

    fun createWindow(useCustomTitlebar: Boolean) {
        windows.add(
            RotatingBallWindow.createWindow(
                app,
                "Window ${windows.count()}",
                useCustomTitlebar,
            ),
        )
    }

    private fun setPaused(value: Boolean) {
//        mainWindow()?.displayLink?.setRunning(!value)
    }

//    private fun mainWindow(): RotatingBallWindow? {
//        return windows.find { window ->
//            window.window.isMain
//        }
//    }
//
//    private fun changeCurrentWindowSize(delta: LogicalPixels) {
//        mainWindow()?.let { window ->
//            val currentOrigin = window.window.origin
//            val currentSize = window.window.size
//            // todo check display bounds
//            // todo check min and max size
//            window.window.setRect(
//                origin = LogicalPoint(
//                    currentOrigin.x - delta / 2.0,
//                    currentOrigin.y - delta / 2.0,
//                ),
//                size = LogicalSize(
//                    currentSize.width + delta,
//                    currentSize.height + delta,
//                ),
//                animateTransition = true,
//            )
//        }
//    }

    private fun killWindow(window: RotatingBallWindow) {
        windows.remove(window)
        window.close()
    }

    override fun close() {
        windows.forEach {
            it.close()
        }
    }
}

fun main(args: Array<String>) {
    Logger.info { runtimeInfo() }
    KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Debug)
    val app = Application(ApplicationConfig())
    ApplicationState(app).use { state ->
        state.createWindow(useCustomTitlebar = true)
        app.runEventLoop()
    }
}
