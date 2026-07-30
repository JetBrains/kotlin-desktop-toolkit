package org.jetbrains.desktop.sample.compose

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogLevel
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.MetalCommandQueue
import org.jetbrains.desktop.macos.MetalDevice
import org.jetbrains.desktop.macos.PhysicalSize
import org.jetbrains.desktop.macos.TitlebarConfiguration
import org.jetbrains.desktop.macos.Window
import org.jetbrains.desktop.macos.WindowEvent
import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.desktop.sample.macos.SkikoWindow
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.FontMgr
import org.jetbrains.skia.Paint
import org.jetbrains.skia.Rect
import org.jetbrains.skia.paragraph.Alignment
import org.jetbrains.skia.paragraph.FontCollection
import org.jetbrains.skia.paragraph.ParagraphBuilder
import org.jetbrains.skia.paragraph.ParagraphStyle
import org.jetbrains.skia.paragraph.TextStyle

/**
 * Scaffold for hosting Compose content inside a kotlin-desktop-toolkit window.
 *
 * The window is a plain [SkikoWindow]: kotlin-desktop-toolkit owns the native window, the Metal view
 * and the display link, and hands us a Skia [Canvas] once per frame. That canvas is the seam where a
 * Compose scene will be rendered, and [handleEvent] is the seam where input will be forwarded to it.
 * For now both just draw and log a placeholder — no Compose dependency is wired up yet.
 */
class ComposeWindow(
    device: MetalDevice,
    queue: MetalCommandQueue,
    windowParams: Window.WindowParams,
) : SkikoWindow(device, queue, windowParams) {
    private val fontCollection = FontCollection().apply { setDefaultFontManager(FontMgr.default) }

    override fun Canvas.draw(size: PhysicalSize, time: Long) {
        val scale = window.scaleFactor().toFloat()

        Paint().use { paint ->
            paint.color = BACKGROUND_COLOR
            drawRect(Rect.makeWH(size.width.toFloat(), size.height.toFloat()), paint)
        }

        // TODO: render a ComposeScene here instead of the placeholder label.
        Paint().use { labelPaint ->
            labelPaint.color = LABEL_COLOR
            labelPaint.isAntiAlias = true
            val textStyle = TextStyle().apply {
                setFontSize(16f * scale)
                setFontFamilies(arrayOf("Helvetica Neue", "sans-serif"))
                setForeground(labelPaint)
            }
            ParagraphBuilder(ParagraphStyle().apply { alignment = Alignment.CENTER }, fontCollection).use { builder ->
                builder.pushStyle(textStyle)
                builder.addText("Compose content goes here")
                builder.build().use { paragraph ->
                    paragraph.layout(size.width.toFloat())
                    paragraph.paint(this, 0f, (size.height.toFloat() - paragraph.height) / 2f)
                }
            }
        }
    }

    companion object {
        private const val BACKGROUND_COLOR = 0xFF1E1F22.toInt()
        private const val LABEL_COLOR = 0xFFBCBEC4.toInt()

        fun createWindow(device: MetalDevice, queue: MetalCommandQueue, title: String, origin: LogicalPoint): ComposeWindow {
            return ComposeWindow(
                device,
                queue,
                Window.WindowParams(
                    origin = origin,
                    size = LogicalSize(800.0, 600.0),
                    title = title,
                    titlebarConfiguration = TitlebarConfiguration.Regular,
                ),
            )
        }
    }
}

class ApplicationState : AutoCloseable {
    private val windows = mutableListOf<ComposeWindow>()

    private val device: MetalDevice by lazy { MetalDevice.create() }
    private val queue: MetalCommandQueue by lazy { MetalCommandQueue.create(device) }

    fun createWindow() {
        windows.add(ComposeWindow.createWindow(device, queue, "Compose Sample", LogicalPoint(100.0, 100.0)))
    }

    fun handleEvent(event: Event): EventHandlerResult {
        if (event is Event.WindowCloseRequest) {
            windows.find { it.window.windowId() == event.windowId }?.let { window ->
                windows.remove(window)
                window.close()
            }
            if (windows.isEmpty()) {
                Application.stopEventLoop()
            }
            return EventHandlerResult.Stop
        }
        return if (event is WindowEvent) {
            windows.find { it.window.windowId() == event.windowId }?.handleEvent(event) ?: EventHandlerResult.Continue
        } else {
            EventHandlerResult.Continue
        }
    }

    override fun close() {
        windows.forEach { it.close() }
        windows.clear()
        queue.close()
        device.close()
    }
}

fun main() {
    KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Info)
    Logger.info { runtimeInfo() }
    GrandCentralDispatch.startOnMainThread {
        Application.init(Application.ApplicationConfig())
        ApplicationState().use { state ->
            state.createWindow()
            Application.runEventLoop { event -> state.handleEvent(event) }
        }
        GrandCentralDispatch.close()
    }
}
