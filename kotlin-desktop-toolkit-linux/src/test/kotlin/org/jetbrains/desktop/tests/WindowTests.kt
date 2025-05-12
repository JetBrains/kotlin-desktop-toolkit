package org.jetbrains.desktop.tests

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.KotlinDesktopToolkit
import org.jetbrains.desktop.linux.LogicalSize
import org.jetbrains.desktop.linux.WindowParams
import org.junit.jupiter.api.Test

class WindowTests {
    @Test
    fun smokeTest() {
        KotlinDesktopToolkit.init()
        val app = Application()
        val window1 = app.createWindow(WindowParams(windowId = 0, appId = "org.jetbrains.desktop.linux.tests", title = "Test Hello1"))
        val window2 = app.createWindow(
            WindowParams(
                windowId = 0,
                appId = "org.jetbrains.desktop.linux.tests",
                title = "Hello2",
                size = LogicalSize(200.0f, 300.0f),
                forceClientSideDecoration = true,
                forceSoftwareRendering = true,
            ),
        )
        window1.close()
        window2.close()
    }
}
