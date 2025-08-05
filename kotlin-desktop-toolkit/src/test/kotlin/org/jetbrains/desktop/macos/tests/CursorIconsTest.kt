package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Cursor
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.Window
import kotlin.test.Test
import kotlin.test.assertEquals

class CursorIconsTest : KDTApplicationTestBase() {
    @Test
    fun interateCursorIconsTest() {
        val window1 = ui {
            Window.create(origin = LogicalPoint(100.0, 200.0), title = "Hello1")
        }
        Cursor.Icon.entries.forEach { icon ->
            ui {
                Cursor.icon = icon
            }
            val actualIcon = ui {
                Cursor.icon
            }
            assertEquals(actualIcon, icon)
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
        assertEquals(actualHidden, true)
        ui {
            Cursor.hidden = false
        }
        val actualVisible = ui {
            !Cursor.hidden
        }
        assertEquals(actualVisible, true)
    }
}
