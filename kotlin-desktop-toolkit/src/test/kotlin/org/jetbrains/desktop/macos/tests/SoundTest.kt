package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Sound
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.assertTrue

@EnabledOnOs(OS.MAC)
internal class SoundTest : KDTApplicationTestBase() {
    @Test
    fun `test play system sounds`() {
        // Test a few common system sounds
        // Note: Available sounds vary by macOS version
        val soundNames = listOf(
            "Basso", "Blow", "Bottle", "Frog", "Funk", "Glass",
            "Hero", "Morse", "Ping", "Pop", "Purr", "Sosumi",
            "Submarine", "Tink",
        )

        ui {
            // At least one of these should exist and play successfully
            val results = soundNames.map { soundName ->
                try {
                    val result = Sound.playNamed(soundName)
                    result
                } catch (e: Exception) {
                    // Some sounds may not be available on all systems
                    false
                }
            }
            assertTrue(results.all { it }, "At least one system sound should play successfully")
        }
    }
}
