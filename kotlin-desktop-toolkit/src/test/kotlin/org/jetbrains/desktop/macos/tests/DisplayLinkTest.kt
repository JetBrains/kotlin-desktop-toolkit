package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.DisplayLink
import org.jetbrains.desktop.macos.Screen
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.util.concurrent.TimeUnit
import kotlin.test.assertTrue

@EnabledOnOs(OS.MAC)
class DisplayLinkTest : KDTApplicationTestBase() {
    @Test
    @Timeout(value = 60, unit = TimeUnit.SECONDS)
    fun `repeatedly create display link for window's screen`() {
        val window = createWindowAndEnsureItsFocused("DisplayLinkTest")
        var counterSuccess = 0
        var counterFailure = 0
        try {
            repeat(1000) {
                ui {
                    if (window.isVisible) {
                        val screenId = window.screenId()
                        println("Captured screenID: $screenId")
                        Thread.sleep(1)
                        println("Actual screens: ${Screen.allScreens()}")
                        val displayLink = DisplayLink.create(screenId, onNextFrame = {})
                        if (displayLink == null) {
                            counterFailure += 1
                            return@ui
                        }
                        counterSuccess += 1
                        displayLink.setRunning(true)
                        assertTrue(displayLink.isRunning(), "DisplayLink should be running after setRunning(true)")
                        displayLink.setRunning(false)
                        displayLink.close()
                    } else {
                        counterFailure += 1
                    }
                }
            }
        } finally {
            println("DisplayLinkTest: success=$counterSuccess, failure=$counterFailure")
            ui { window.close() }
        }
    }
}
