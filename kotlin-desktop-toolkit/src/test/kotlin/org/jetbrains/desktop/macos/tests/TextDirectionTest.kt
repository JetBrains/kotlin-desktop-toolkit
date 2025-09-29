package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.TextDirection
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.Test
import kotlin.test.assertEquals

@EnabledOnOs(OS.MAC)
class TextDirectionTest : KDTApplicationTestBase() {
    @Test
    fun applicationTextDirection() {
        val applicationTextDirection = ui {
            Application.textDirection
        }
        assertEquals(TextDirection.LeftToRight, applicationTextDirection)
    }

    @Test
    fun windowTextDirection() {
        val window = ui {
            Window.create(title = "TextDirection Test Window")
        }

        val windowTextDirection = ui {
            window.textDirection
        }

        assertEquals(TextDirection.LeftToRight, windowTextDirection)

        ui {
            window.close()
        }
    }
}
