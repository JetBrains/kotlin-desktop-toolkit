package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.*
import org.jetbrains.skia.*
import java.lang.Thread.sleep
import kotlin.concurrent.thread
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin
import kotlin.time.TimeSource

class SkiaWindow(device: MetalDevice,
                 val queue: MetalCommandQueue,
                 title: String,
                 x: Float, y: Float) {
    val window = Window.create(title, x, y, onResize = {
        draw()
    })
    val displayLink = DisplayLink.createForWindow(window, onNextFrame = {
        draw()
    })
    val directContext = DirectContext.makeMetal(device.pointer.address(), queue.pointer.address())
    var view: MetalView = MetalView.create(device)
    val creationTime = TimeSource.Monotonic.markNow()

    init {
        view.attachToWindow(window)
    }

    fun draw() {
        val size = view.size()
        view.nextTexture().use { texture ->
//             sleep(100) // uncomment this to check window resize quality
            BackendRenderTarget.makeMetal(size.width.toInt(), size.height.toInt(), texture.pointer.address()).use { renderTarget ->
                Surface.makeFromBackendRenderTarget(
                    context = directContext,
                    origin = SurfaceOrigin.TOP_LEFT,
                    colorFormat = SurfaceColorFormat.BGRA_8888,
                    colorSpace = null,
                    surfaceProps = null,
                    rt = renderTarget
                )!!.use { surface ->
                    val time = creationTime.elapsedNow().inWholeMilliseconds
                    surface.canvas.paint(surface.width, surface.height, time)
                    surface.flushAndSubmit()
                }
            }
            queue.commit()
            view.present()
        }
    }

    fun Canvas.drawSpiningCircle(width: Int, height: Int, t: Long) = let { canvas ->
        val angle = (t / 2000f) * 2f * PI
        val r = width / 4
        val x = r * sin(angle).toFloat() + width / 2f
        val y = r * cos(angle).toFloat() + height / 2f
        Paint().use { paint ->
            paint.color = Color.GREEN
            canvas.drawCircle(x, y, 30f, paint)
        }
    }

    fun Canvas.drawWindowBorders(width: Int, height: Int, t: Long) = let { canvas ->
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

    fun Canvas.paint(width: Int, height: Int, t: Long) = let { canvas ->
        canvas.clear(0xFF264653.toInt());
        drawSpiningCircle(width, height, t)
        drawWindowBorders(width, height, t)
    }
}

class ApplicationState {
    val windows = mutableListOf<SkiaWindow>()

    val device: MetalDevice by lazy {
        MetalDevice.create()
    }

    val queue by lazy {
        MetalCommandQueue.create(device)
    }

    fun createWindow() {
        windows.add(SkiaWindow(device, queue, "Window ${windows.count()}", 200f, 200f))
    }

    fun setPaused(value: Boolean) {
        windows.forEach {
            it.displayLink.setPaused(value)
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
                    "Pause",
                    keystroke = Keystroke(key = "p", modifiers = Modifiers(command = true)),
                    perform = { setPaused(true) }
                ),
                AppMenuItem.Action(
                    "Run",
                    keystroke = Keystroke(key = "r", modifiers = Modifiers(command = true)),
                    perform = { setPaused(false) }
                ),
            ),
            AppMenuItem.SubMenu(
                title = "Window",
                specialTag = "Window"
            )
        )
    }
}

fun main() {
    printRuntimeInfo()
    Application.initWithConfig(Application.Config())
    val state = ApplicationState()
    AppMenuManager.setMainMenu(state.buildMenu())
    Window.create("First", 200f, 200f)
    Application.runEventLoop()
}