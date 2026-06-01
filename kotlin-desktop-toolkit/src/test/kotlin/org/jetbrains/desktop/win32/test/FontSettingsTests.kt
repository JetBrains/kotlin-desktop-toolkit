package org.jetbrains.desktop.win32.test

import org.jetbrains.desktop.win32.FontSmoothing
import org.jetbrains.desktop.win32.FontSmoothingOrientation
import org.jetbrains.desktop.win32.FontSmoothingType
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.jetbrains.desktop.win32.getFontSmoothingContrast
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.nio.file.Path

@EnabledOnOs(OS.WINDOWS)
class FontSettingsTests {
    @Test
    fun `font smoothing settings can be queried`() {
        KotlinDesktopToolkit.init(
            libraryFolderPath = Path.of(System.getProperty("kdt.win32.library.folder.path")!!),
        )
        FontSmoothing.getCurrent()
        FontSmoothingType.getCurrent()
        FontSmoothingOrientation.getCurrent()
        val contrast = getFontSmoothingContrast()
        assert(contrast in 1000..2200) { "Font smoothing contrast out of expected range [1000, 2200]: $contrast" }
    }
}
