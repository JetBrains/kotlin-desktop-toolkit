package org.jetbrains.kwm.sample

import org.jetbrains.kwm.Point
import org.jetbrains.kwm.Size
import org.jetbrains.kwm.macos.*
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import java.lang.AutoCloseable
import kotlin.concurrent.thread
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

class RotatingBallWindow(device: MetalDevice,
                         queue: MetalCommandQueue,
                         title: String,
                         position: Point): SkikoWindow(device, queue, title, position) {

    fun Canvas.drawSpiningCircle(size: Size, t: Long) = let { canvas ->
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

    fun Canvas.drawWindowBorders(size: Size, t: Long) {
        val canvas = this
        val width = size.width.toFloat()
        val height = size.height.toFloat()
        val scale = 2f // todo fixme!
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

    override fun Canvas.draw(size: Size, t: Long) {
        val canvas = this
        canvas.clear(0xFF264653.toInt());
        drawSpiningCircle(size, t)
        drawWindowBorders(size, t)
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
        windows.add(RotatingBallWindow(device, queue, "Window ${windows.count()}", Point(200.0, 200.0)))
    }

    fun setPaused(value: Boolean) {
        windows.forEach {
            it.displayLink.setPaused(value)
        }
    }

    fun handleEvent(event: Event): EventHandlerResult {
        val eventWindowId = event.windowId()
        val window = windows.find {
            it.window.windowId() == eventWindowId
        }
        println("Application got event: $event window: $window")
        return EventHandlerResult.Skipped
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
                    "Pause",
                    keystroke = Keystroke(key = "p", modifiers = Modifiers(command = true)),
                    perform = { setPaused(true) }
                ),
                AppMenuItem.Action(
                    "Run",
                    keystroke = Keystroke(key = "r", modifiers = Modifiers(command = true)),
                    perform = { setPaused(false) }
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
                title = "Window",
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
        Window.create("First", 200f, 200f).use { firstWindow ->
            AppMenuManager.setMainMenu(state.buildMenu())

            println("first window id: ${firstWindow.windowId()}")

            Application.runEventLoop { event ->
                state.handleEvent(event)
            }
        }
    }
}