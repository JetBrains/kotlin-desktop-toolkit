package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.TitlebarConfiguration
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

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

    @Test
    fun isResizableTest() {
        val window = ui {
            Window.create(title = "ResizableTest", isResizable = true)
        }

        ui {
            assertTrue(window.isResizable, "Window should be resizable by default")

            window.isResizable = false
            assertFalse(window.isResizable, "Window should not be resizable after setting to false")

            window.isResizable = true
            assertTrue(window.isResizable, "Window should be resizable after setting back to true")
        }

        ui {
            window.close()
        }
    }

    @Test
    fun `window is created with expected size`() {
        val windowWithCustomTitlebar = ui {
            Window.create(
                title = "Window with custom titlebar",
                size = LogicalSize(100.0, 100.0),
                titlebarConfiguration = TitlebarConfiguration.Custom(titlebarHeight = 10.0),
                isResizable = false,
            )
        }

        val windowWithRegularTitlebar = ui {
            Window.create(
                title = "Window with regular titlebar",
                size = LogicalSize(100.0, 100.0),
                titlebarConfiguration = TitlebarConfiguration.Regular,
                isResizable = false,
            )
        }

        ui {
            assertEquals(windowWithCustomTitlebar.size, LogicalSize(100.0, 100.0), "A custom titlebar window should have the expected size")
            assertEquals(windowWithRegularTitlebar.size, LogicalSize(100.0, 100.0), "Regular titlebar window should have the expected size")
        }

        ui {
            windowWithCustomTitlebar.close()
            windowWithRegularTitlebar.close()
        }
    }
}
