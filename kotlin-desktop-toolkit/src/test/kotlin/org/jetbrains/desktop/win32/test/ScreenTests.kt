package org.jetbrains.desktop.win32.test

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
            assertTrue(screen.size.width > 0f, "Screen width should be positive, got: ${screen.size.width}")
            assertTrue(screen.size.height > 0f, "Screen height should be positive, got: ${screen.size.height}")
            assertTrue(screen.scale > 0f, "Screen scale should be positive, got: ${screen.scale}")
            assertTrue(
                screen.maximumFramesPerSecond > 0,
                "Screen maximumFramesPerSecond should be positive, got: ${screen.maximumFramesPerSecond}",
            )
        }

        assertEquals(
            1,
            screens.count { it.isPrimary },
            "Expected exactly one primary screen, got: ${screens.count { it.isPrimary }}",
        )
    }
}
