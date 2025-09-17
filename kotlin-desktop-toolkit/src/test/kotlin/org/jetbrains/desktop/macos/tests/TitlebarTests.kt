package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.TitlebarConfiguration
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.Test

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
