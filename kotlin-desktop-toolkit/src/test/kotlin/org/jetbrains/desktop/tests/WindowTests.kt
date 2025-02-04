package org.jetbrains.desktop.tests

import org.jetbrains.desktop.LogicalPoint
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.Test

class WindowTests {
    @Test
    fun smokeTest() {
        KotlinDesktopToolkit.init()
        val window1 = GrandCentralDispatch.dispatchOnMainSync {
            Window.create(origin = LogicalPoint(100.0, 200.0), title = "Hello1")
        }
        val window2 = GrandCentralDispatch.dispatchOnMainSync {
            Window.create(origin = LogicalPoint(200.0, 300.0), title = "Hello2")
        }
        GrandCentralDispatch.dispatchOnMainSync {
            window1.close()
            window2.close()
        }
    }
}