package org.jetbrains.desktop.tests

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.WindowParams
import org.junit.jupiter.api.Test

class WindowTests {
    @Test
    fun smokeTest() {
        KotlinDesktopToolkit.init()
        Application.init()
        Application.runEventLoop()
        val window1 = Application.createWindow(WindowParams(title = "Test Hello1"))
        val window2 = Application.createWindow(
            WindowParams(
                title = "Hello2",
                size = PhysicalSize(200, 300),
            ),
        )
        window1.show()
        window2.show()
        window1.close()
        window2.close()
    }
}
