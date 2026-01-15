package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Screen
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class ScreenTests : KDTApplicationTestBase() {
    @Test
    fun `allScreens returns at least one screen with valid uuid`() {
        val allScreens = ui { Screen.allScreens() }
        assertTrue(allScreens.screens.isNotEmpty(), "Expected at least one screen")

        for (screen in allScreens.screens) {
            assertTrue(screen.uuid.isNotEmpty(), "Screen UUID should not be empty")
            // UUID format: XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
            assertTrue(
                screen.uuid.matches(Regex("[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}")),
                "Screen UUID should match UUID format, got: ${screen.uuid}",
            )
        }
    }
}
