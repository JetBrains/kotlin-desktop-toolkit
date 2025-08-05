package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Window
import kotlin.test.Ignore
import kotlin.test.Test

class WindowTests: KDTApplicationTestBase() {
    @Test
    fun smokeTest() {
        val window1 = ui {
            Window.create(origin = LogicalPoint(100.0, 200.0), title = "Hello1")
        }
        val window2 = ui {
            Window.create(origin = LogicalPoint(200.0, 300.0), title = "Hello2")
        }
        ui {
            window1.close()
            window2.close()
        }
    }
}
