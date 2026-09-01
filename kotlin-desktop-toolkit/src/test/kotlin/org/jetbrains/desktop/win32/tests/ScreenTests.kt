package org.jetbrains.desktop.win32.tests

import org.assertj.core.api.Assertions.assertThat
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.Screen
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.nio.file.Path

@EnabledOnOs(OS.WINDOWS)
class ScreenTests {
    @Test
    fun `allScreens returns at least one screen with valid info`() {
        KotlinDesktopToolkit.init(
            libraryFolderPath = Path.of(System.getProperty("kdt.win32.library.folder.path")!!),
        )
        val screens = Screen.allScreens()
        assertTrue(screens.isNotEmpty(), "Expected at least one screen")

        for (screen in screens) {
            assertThat(screen.size.width).`as`("Screen width").isPositive()
            assertThat(screen.size.height).`as`("Screen height").isPositive()
            assertThat(screen.scale).`as`("Screen scale").isPositive()
            assertThat(screen.maximumFramesPerSecond).`as`("Screen maximumFramesPerSecond").isPositive()
        }

        assertEquals(
            1,
            screens.count { it.isPrimary },
            "Expected exactly one primary screen, got: ${screens.count { it.isPrimary }}",
        )
    }
}
