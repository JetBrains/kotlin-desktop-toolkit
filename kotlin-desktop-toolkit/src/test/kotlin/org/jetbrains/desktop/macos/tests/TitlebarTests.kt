package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.LogicalSize
import org.jetbrains.desktop.macos.TitlebarConfiguration
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.util.concurrent.TimeUnit
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

@EnabledOnOs(OS.MAC)
class TitlebarTests : KDTApplicationTestBase() {
    @Test
    fun windowWithRegularTitlebarTest() {
        val window = ui {
            Window.create(titlebarConfiguration = TitlebarConfiguration.Regular)
        }
        ui {
            window.close()
        }
    }

    @Test
    fun windowWithCustomTitlebarTest() {
        val window = ui {
            Window.create(
                titlebarConfiguration = TitlebarConfiguration.Custom(titlebarHeight = 42.0),
            )
        }
        ui {
            window.close()
        }
    }

    @Test
    fun `window noop switch`() {
        switchTitlebarHelper(
            from = TitlebarConfiguration.Regular,
            to = TitlebarConfiguration.Regular,
        )
    }

    @Test
    fun `window changes it's titlebar height`() {
        switchTitlebarHelper(
            from = TitlebarConfiguration.Custom(titlebarHeight = 42.0),
            to = TitlebarConfiguration.Custom(titlebarHeight = 22.0),
        )
    }

    @Test
    fun `window switch to custom titlebar`() {
        switchTitlebarHelper(
            from = TitlebarConfiguration.Regular,
            to = TitlebarConfiguration.Custom(titlebarHeight = 22.0),
        )
    }

    @Test
    fun `window switch to regular titlebar`() {
        switchTitlebarHelper(
            from = TitlebarConfiguration.Custom(titlebarHeight = 22.0),
            to = TitlebarConfiguration.Regular,
        )
    }

    @Test
    fun `window keeps its size when switching to custom titlebar`() {
        val initialSize = LogicalSize(640.0, 480.0)
        val window = ui {
            Window.create(
                size = initialSize,
                titlebarConfiguration = TitlebarConfiguration.Regular,
                isResizable = false,
            )
        }
        ui {
            assertEquals(initialSize, window.size, "Window should have the requested size before switching titlebar")
            window.setTitlebarConfiguration(TitlebarConfiguration.Custom(titlebarHeight = 30.0))
            assertEquals(initialSize, window.size, "Window size should be preserved after switching to custom titlebar")
        }
        ui {
            window.close()
        }
    }

    @Test
    fun `left inset is zero for regular titlebar`() {
        val window = ui {
            Window.create(titlebarConfiguration = TitlebarConfiguration.Regular)
        }
        ui {
            assertEquals(0.0, window.getTitlebarLeftInset(), "Left inset should be zero for the regular titlebar")
            window.close()
        }
    }

    @Test
    fun `left inset is positive for custom titlebar`() {
        val window = ui {
            Window.create(titlebarConfiguration = TitlebarConfiguration.Custom(titlebarHeight = 42.0))
        }
        ui {
            val leftInset = window.getTitlebarLeftInset()
            assertTrue(leftInset > 0.0, "Left inset should be positive for a custom titlebar, got $leftInset")
            window.close()
        }
    }

    @Test
    fun `left inset updates on titlebar configuration switch`() {
        val window = ui {
            Window.create(titlebarConfiguration = TitlebarConfiguration.Regular)
        }
        ui {
            assertEquals(0.0, window.getTitlebarLeftInset(), "Left inset should be zero before switching to custom titlebar")

            window.setTitlebarConfiguration(TitlebarConfiguration.Custom(titlebarHeight = 42.0))
            val leftInset = window.getTitlebarLeftInset()
            assertTrue(leftInset > 0.0, "Left inset should be positive after switching to custom titlebar, got $leftInset")

            window.setTitlebarConfiguration(TitlebarConfiguration.Regular)
            assertEquals(0.0, window.getTitlebarLeftInset(), "Left inset should be zero after switching back to regular titlebar")

            window.close()
        }
    }

    @Timeout(value = 60, unit = TimeUnit.SECONDS)
    @Test
    fun `left inset is zero in full screen`() {
        val window = ui {
            Window.create(titlebarConfiguration = TitlebarConfiguration.Custom(titlebarHeight = 42.0))
        }
        ui {
            window.makeKeyAndOrderFront()
        }
        awaitEventOfType<Event.WindowChangedOcclusionState> { it.windowId == window.windowId() && it.isVisible }

        ui {
            window.toggleFullScreen()
        }
        awaitEventOfType<Event.WindowFullScreenToggle> { it.windowId == window.windowId() && it.isFullScreen }
        ui {
            assertEquals(0.0, window.getTitlebarLeftInset(), "Left inset should be zero in full screen")
        }

        ui {
            window.toggleFullScreen()
        }
        awaitEventOfType<Event.WindowFullScreenToggle> { it.windowId == window.windowId() && !it.isFullScreen }
        ui {
            val leftInset = window.getTitlebarLeftInset()
            assertTrue(leftInset > 0.0, "Left inset should be positive after exiting full screen, got $leftInset")
            window.close()
        }
    }

    fun switchTitlebarHelper(from: TitlebarConfiguration, to: TitlebarConfiguration) {
        val window = ui {
            Window.create(
                titlebarConfiguration = from,
            )
        }
        ui {
            window.setTitlebarConfiguration(to)
        }
        ui {
            window.close()
        }
    }
}
