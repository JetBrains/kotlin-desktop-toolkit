package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.LogicalSize
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
            val window1 = app.createWindow(WindowParams(title = "Test Hello1"))
            val window2 = app.createWindow(
                WindowParams(
                    title = "Hello2",
                    size = LogicalSize(200f, 300f),
                ),
            )
            window1.show()
            window2.show()
            window1.close()
            window2.close()
        }
    }
}
