package org.jetbrains.kwm.sample

import org.jetbrains.kwm.LogicalPixels
import org.jetbrains.kwm.LogicalPoint
import org.jetbrains.kwm.LogicalSize
import org.jetbrains.kwm.PhysicalSize
import org.jetbrains.kwm.macos.*
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import java.lang.AutoCloseable
import java.lang.Thread.sleep
import kotlin.concurrent.thread
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

class RotatingBallWindow(device: MetalDevice,
                         queue: MetalCommandQueue,
                         title: String,
                         position: LogicalPoint): SkikoWindow(device, queue, title, position) {

    private var cursorPosition: LogicalPoint? = null

    override fun handleEvent(event: Event): EventHandlerResult {
        return if (super.handleEvent(event) == EventHandlerResult.Continue) {
            when (event) {
                is Event.MouseMoved -> {
                    cursorPosition = event.point
                    EventHandlerResult.Stop
                }
                else -> EventHandlerResult.Continue
            }
        } else {
            EventHandlerResult.Stop
        }
    }

    private fun Canvas.drawSpiningCircle(size: PhysicalSize, t: Long) = let { canvas ->
        val width = size.width.toFloat()
        val height = size.height.toFloat()
        val angle = (t / 2000f) * 2f * PI
        val r = width / 4
        val x = r * sin(angle).toFloat() + width / 2f
        val y = r * cos(angle).toFloat() + height / 2f
        Paint().use { paint ->
            paint.color = Color.GREEN
            canvas.drawCircle(x, y, 30f, paint)
        }
    }

    private fun Canvas.drawWindowBorders(size: PhysicalSize, t: Long) {
        val canvas = this
        val width = size.width.toFloat()
        val height = size.height.toFloat()
        val scale = scale()
        Paint().use { paint ->

            val barSize = 3 * scale

            // left
            paint.color = 0xFFe76f51.toInt()
            canvas.drawRect(Rect.makeXYWH(0f, 0f, barSize, 100 * scale), paint)
            canvas.drawRect(Rect.makeXYWH(0f, height / 2 - 50 * scale, barSize, 100 * scale), paint)
            canvas.drawRect(Rect.makeXYWH(0f, height - 100 * scale, barSize, 100 * scale), paint)

            // top
            paint.color = 0xFF2a9d8f.toInt()
            canvas.drawRect(Rect.makeXYWH(0f, 0f, 100 * scale, barSize), paint)
            canvas.drawRect(Rect.makeXYWH(width / 2 - 50 * scale, 0f, 100 * scale, barSize), paint)
            canvas.drawRect(Rect.makeXYWH(width - 100 * scale, 0f, 100 * scale, barSize), paint)

            // right
            paint.color = 0xFFe9c46a.toInt()
            canvas.drawRect(Rect.makeXYWH(width - barSize, 0f, barSize, 100 * scale), paint)
            canvas.drawRect(Rect.makeXYWH(width - barSize, height / 2 - 50 * scale, barSize, 100 * scale), paint)
            canvas.drawRect(Rect.makeXYWH(width - barSize, height - 100 * scale, barSize, 100 * scale), paint)

            // bottom
            paint.color = 0xFFFFFFFF.toInt()
            canvas.drawRect(Rect.makeXYWH(0f, height - barSize, 100 * scale, barSize), paint)
            canvas.drawRect(Rect.makeXYWH(width / 2 - 50 * scale, height - barSize, 100 * scale, barSize), paint)
            canvas.drawRect(Rect.makeXYWH(width - 100 * scale, height - barSize, 100 * scale, barSize), paint)
        }
    }

    private fun Canvas.drawCursor(size: PhysicalSize, t: Long) {
        val canvas = this
        val scale = scale()
        cursorPosition?.let { curs ->
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

    override fun Canvas.draw(size: PhysicalSize, t: Long) {
        val canvas = this
        canvas.clear(0xFF264653.toInt());
        drawSpiningCircle(size, t)
        drawWindowBorders(size, t)
        drawCursor(size, t)
    }
}

class ApplicationState: AutoCloseable {
    val windows = mutableListOf<RotatingBallWindow>()

    val device: MetalDevice by lazy {
        MetalDevice.create()
    }

    val queue by lazy {
        MetalCommandQueue.create(device)
    }

    fun createWindow() {
        windows.add(RotatingBallWindow(device, queue, "Window ${windows.count()}", LogicalPoint(0.0, 0.0)))
    }

    fun setPaused(value: Boolean) {
        windows.forEach {
            it.displayLink.setRunning(!value)
        }
    }

    private fun mainWindow(): RotatingBallWindow? {
        return windows.find { window ->
            window.window.isMain
        }
    }

    private fun changeCurrentWindowSize(delta: LogicalPixels) {
        mainWindow()?.let { window ->
            val currentOrigin = window.window.origin
            val currentSize = window.window.size
            // todo check display bounds
            // todo check min and max size
            window.window.setRect(
                origin = LogicalPoint(currentOrigin.x - delta / 2.0,
                                      currentOrigin.y - delta / 2.0),
                size = LogicalSize(currentSize.width + delta,
                                   currentSize.height + delta),
                animateTransition = true)
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
        val eventWindowId = event.windowId()

        return when (event) {
            is Event.WindowCloseRequest -> {
                windows.find {
                    it.window.windowId() == eventWindowId
                }?.let { window ->
                    windows.remove(window)
                    window.close()
                } ?: run {
                    println("Can't find window for $event")
                }
                EventHandlerResult.Stop
            }
            else -> {
                val window = windows.find {
                    it.window.windowId() == eventWindowId
                }
                window?.let {
                    window.handleEvent(event)
                } ?: EventHandlerResult.Continue
            }
        }
    }

    fun buildMenu(): AppMenuStructure {
        return AppMenuStructure(
            AppMenuItem.SubMenu(
                title = "App", // Ignored
                AppMenuItem.Action(
                    "New Window",
                    keystroke = Keystroke(key = "n", modifiers = Modifiers(command = true)),
                    perform = { createWindow() }
                ),
                AppMenuItem.Action(
                    "Quit1",
                    keystroke = Keystroke(key = "q", modifiers = Modifiers(command = true)),
                    perform = { Application.stopEventLoop() }
                ),
                AppMenuItem.Action(
                    "Quit2",
                    keystroke = Keystroke(key = "w", modifiers = Modifiers(command = true)),
                    perform = {
                        thread {
                            // we shouldn't call this function on main thread because it block it
                            Runtime.getRuntime().exit(0)
                        }
                    }
                ),
            ),
            AppMenuItem.SubMenu(
                title = "View",
                specialTag = "View"
            ),
            AppMenuItem.SubMenu(
                title = "Animation",
                AppMenuItem.Action(
                    title = "Pause",
                    keystroke = Keystroke(key = "p", modifiers = Modifiers(command = true)),
                    perform = { setPaused(true) }
                ),
                AppMenuItem.Action(
                    title = "Run",
                    keystroke = Keystroke(key = "r", modifiers = Modifiers(command = true)),
                    perform = { setPaused(false) }
                ),
            ),
            AppMenuItem.SubMenu(
                title = "Displays",
                AppMenuItem.Action(
                    title = "List Displays",
                    keystroke = Keystroke(key = "d", modifiers = Modifiers(command = true)),
                    perform = { println(Screen.allScreens()) }
                ),
            ),
            AppMenuItem.SubMenu(
                title = "Window",
                AppMenuItem.Action(
                    title = "Increase Size",
                    keystroke = Keystroke(key = "+", modifiers = Modifiers(command = true)),
                    perform = { changeCurrentWindowSize(50.0) }
                ),
                AppMenuItem.Action(
                    title = "Drecrease Size",
                    keystroke = Keystroke(key = "-", modifiers = Modifiers(command = true)),
                    perform = { changeCurrentWindowSize(-50.0) }
                ),
                specialTag = "Window"
            )
        )
    }

    override fun close() {
        windows.forEach {
            it.close()
        }
        queue.close()
        device.close()
    }
}

fun main() {
    printRuntimeInfo()
    Application.init(Application.Config())
    ApplicationState().use { state ->
        state.createWindow()
        AppMenuManager.setMainMenu(state.buildMenu())
        Application.runEventLoop { event ->
            state.handleEvent(event)
        }
    }
}