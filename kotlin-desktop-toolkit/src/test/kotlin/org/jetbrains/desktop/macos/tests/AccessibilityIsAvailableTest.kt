package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Robot
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.Test
import kotlin.test.assertTrue

@EnabledOnOs(OS.MAC)
class AccessibilityIsAvailableTest : KDTApplicationTestBase() {
    @Test
    fun `Accessibility is available`() {
        val robot = ui { Robot() }
        try {
            assertTrue { robot.isAccessibilityAllowed() }
        } finally {
            ui { robot.close() }
        }
    }
}
