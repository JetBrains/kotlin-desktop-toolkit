package org.jetbrains.desktop.win32.tests

import org.jetbrains.desktop.win32.Cursor
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.nio.file.Path

@EnabledOnOs(OS.WINDOWS)
class CursorTests {
    @Test
    fun `hide and show act like a stack of hide requests`() {
        KotlinDesktopToolkit.init(
            libraryFolderPath = Path.of(System.getProperty("kdt.win32.library.folder.path")!!),
        )

        val afterFirstHide = Cursor.hide()
        assertEquals(afterFirstHide - 1, Cursor.hide(), "A second hide pushes another request, one step lower")
        assertEquals(afterFirstHide, Cursor.show(), "show pops the last hide, back to one outstanding")
        assertEquals(afterFirstHide + 1, Cursor.show(), "show pops the final hide, back to the baseline")
    }
}
