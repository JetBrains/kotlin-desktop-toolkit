package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Cursor
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Disabled
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS

@EnabledOnOs(OS.MAC)
class CursorIconsTest : KDTApplicationTestBase() {
    @Test
    fun iterateCursorIconsTest() {
        val window1 = ui {
            Window.create(origin = LogicalPoint(100.0, 200.0), title = "Hello1")
        }
        repeat(1000) {
            Cursor.Icon.entries.forEach { icon ->
                val actualIcon = ui {
                    Cursor.icon = icon
                    Cursor.icon
                }
                assertEquals(actualIcon, icon)
            }
        }
        ui {
            window1.close()
        }
    }

    @Disabled("This test is flaky")
    @Test
    fun cursorIconShouldntChangeRandomly() {
        val window1 = ui {
            Window.create(origin = LogicalPoint(100.0, 200.0), title = "Hello1")
        }
        repeat(1000) {
            Cursor.Icon.entries.forEach { icon ->
                ui {
                    Cursor.icon = icon
                }
                val actualIcon = ui {
                    Cursor.icon
                }
                assertEquals(actualIcon, icon)
            }
        }
        ui {
            window1.close()
        }
    }

    @Test
    fun hideAndShowTest() {
        ui {
            Cursor.hidden = true
        }
        val actualHidden = ui {
            Cursor.hidden
        }
        assertEquals(true, actualHidden)
        ui {
            Cursor.hidden = false
        }
        val actualVisible = ui {
            !Cursor.hidden
        }
        assertEquals(true, actualVisible)
    }

    @Test
    fun setHiddenUntilMouseMovesTest() {
        ui {
            Cursor.setHiddenUntilMouseMoves(true)
        }
        // The cursor should be hidden until the mouse moves.
        // We can't easily verify the visual effect, but we can verify the call doesn't crash.
        ui {
            Cursor.setHiddenUntilMouseMoves(false)
        }
    }
}
