package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.Test

@EnabledOnOs(OS.MAC)
class WindowTests : KDTApplicationTestBase() {
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
