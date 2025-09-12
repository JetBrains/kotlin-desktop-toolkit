package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.LogLevel
import org.jetbrains.desktop.win32.Logger
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.WindowId
import org.jetbrains.desktop.win32.WindowParams
import org.jetbrains.desktop.win32.WindowStyle
import org.jetbrains.desktop.win32.WindowSystemBackdropType
import org.jetbrains.desktop.win32.WindowTitleBarKind
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Paint
import org.jetbrains.skia.PaintMode
import org.jetbrains.skia.RRect
import org.jetbrains.skia.skottie.Animation
import org.jetbrains.skia.skottie.AnimationBuilder
import java.lang.AutoCloseable
import kotlin.Array
import kotlin.Float
import kotlin.Long
import kotlin.String
import kotlin.use

fun legoAnimation(): String {
    return object {}.javaClass.getResource("/lego_loader.json")!!.readText()
}

class SkottieWindow(
    app: Application,
    windowParams: WindowParams,
) : SkikoWindowWin32(app, windowParams) {
    companion object {
        fun createWindow(app: Application, windowParams: WindowParams): SkottieWindow {
            return SkottieWindow(app, windowParams)
        }

        private const val ANIMATION_FRAME_COUNT: Int = 151
    }

    private val animation: Animation by lazy {
        AnimationBuilder().use {
            it.buildFromString(legoAnimation())
        }
    }

    override fun Canvas.draw(size: PhysicalSize, scale: Float, time: Long) {
        val canvas = this
        Paint().use { paint ->
            paint.color = 0xFF_FF_FF_FF.toInt()
            paint.mode = PaintMode.FILL
            canvas.drawRRect(
                RRect.makeXYWH(12f, 60f, size.width.toFloat() - 24f, size.height.toFloat() - 72f, 12f),
                paint,
            )
        }
        Paint().use { paint ->
            paint.color = 0x80_00_00_00.toInt()
            paint.mode = PaintMode.STROKE
            canvas.drawRRect(
                RRect.makeXYWH(12f, 60f, size.width.toFloat() - 24f, size.height.toFloat() - 72f, 12f),
                paint,
            )
        }
        val animationDuration = ANIMATION_FRAME_COUNT * animation.fPS
        val frame = (time.toFloat() % animationDuration) / animation.fPS
        animation.seekFrame(frame)
        animation.render(canvas, (size.width.toFloat() / 2) - (animation.width / 2), (size.height.toFloat() / 2) - (animation.height / 2))
        window.requestRedraw()
    }
}

class ApplicationState(private val app: Application) : AutoCloseable {
    private val windows = mutableMapOf<WindowId, SkottieWindow>()

    fun createWindow() {
        val windowParams = WindowParams(
            size = LogicalSize(width = 640f, height = 480f),
            title = "Sample Window",
            style = WindowStyle(
                systemBackdropType = WindowSystemBackdropType.MicaAlt,
                titleBarKind = WindowTitleBarKind.Custom,
            ),
        )

        val window = SkottieWindow.createWindow(app, windowParams)

        windows[window.window.windowId()] = window
        window.window.show()
    }

    fun handleEvent(event: Event, windowId: WindowId): EventHandlerResult {
        val window = windows[windowId] ?: return EventHandlerResult.Continue
        return when (event) {
            is Event.WindowCloseRequest -> {
                window.close()
                windows.remove(windowId)
                if (windows.isEmpty()) {
                    app.stopEventLoop()
                }
                EventHandlerResult.Stop
            }
            else -> window.handleEvent(event)
        }
    }

    override fun close() {
        windows.values.forEach(AutoCloseable::close)
        windows.clear()
    }
}

fun main(args: Array<String>) {
    if (args.isNotEmpty()) {
        Logger.info { "args = $args" }
    }
    Logger.info { runtimeInfo() }
    KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Debug)
    Application().use { app ->
        ApplicationState(app).use { state ->
            app.runEventLoop(
                onStartup = {
                    state.createWindow()
                },
                eventHandler = { windowId, event ->
                    state.handleEvent(event, windowId)
                },
            )
        }
    }
}
