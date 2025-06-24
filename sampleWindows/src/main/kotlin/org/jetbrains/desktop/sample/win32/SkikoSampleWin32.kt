package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.LogLevel
import org.jetbrains.desktop.win32.Logger
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.WindowId
import org.jetbrains.desktop.win32.WindowParams
import org.jetbrains.skia.Canvas
import java.lang.AutoCloseable

class RotatingBallWindow(
    windowParams: WindowParams,
) : SkikoWindowWin32(windowParams) {
    companion object {
        fun createWindow(windowParams: WindowParams): RotatingBallWindow {
            return RotatingBallWindow(windowParams)
        }
    }

    override fun Canvas.draw(size: PhysicalSize, scale: Float, time: Long) {
        //val canvas = this
        //windowContainer.draw(canvas, time, scale, title, editorState)
    }
}

class ApplicationState : AutoCloseable {
    private val windows = mutableMapOf<WindowId, RotatingBallWindow>()

    fun createWindow() {
        val windowParams = WindowParams(
            size = PhysicalSize(width = 640, height = 480),
            title = "Window",
        )

        val window = RotatingBallWindow.createWindow(
            windowParams,
        )

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
                    Application.stopEventLoop()
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
    Application.init()
    ApplicationState().use { state ->
        state.createWindow()
        Application.runEventLoop { windowId, event ->
            state.handleEvent(event, windowId)
        }
    }
}
