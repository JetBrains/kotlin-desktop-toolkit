package org.jetbrains.desktop.win32.test

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.Window
import org.jetbrains.desktop.win32.WindowId
import org.jetbrains.desktop.win32.WindowParams
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS

@EnabledOnOs(OS.WINDOWS)
class WindowTests {
    @Test
    fun smokeTest() {
        KotlinDesktopToolkit.init()
        Application().use { app ->
            val windows = mutableMapOf<WindowId, Window>()
            app.onStartup {
                val window1 = app.createWindow(WindowParams(title = "Test Hello1"))
                val window2 = app.createWindow(
                    WindowParams(
                        title = "Hello2",
                        size = LogicalSize(200f, 300f),
                    ),
                )
                windows[window1.windowId()] = window1
                windows[window2.windowId()] = window2
                window1.show()
                window2.show()
            }
            app.runEventLoop { windowId, event ->
                windows[windowId]?.also { window ->
                    when (event) {
                        is Event.WindowCloseRequest -> {
                            windows.remove(windowId)
                            if (windows.isEmpty()) {
                                app.stopEventLoop()
                            }
                        }
                        else -> window.requestClose()
                    }
                }
                EventHandlerResult.Continue
            }
        }
    }
}
