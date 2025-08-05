package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Appearance
import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.Window
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class AppearanceTest: KDTApplicationTestBase() {
    @Test
    fun smokeTest() {
        val window1 = ui {
            Window.create(title = "Window1")
        }
        val window2 = ui {
            Window.create(title = "Window2")
        }
        val applicationAppearanceBefore = ui {
            val applicationAppearance = Application.appearance
            assertTrue { Appearance.entries.toTypedArray().contains(applicationAppearance) }
            assertNull(window1.overriddenAppearance)
            assertNull(window2.overriddenAppearance)
            window1.overriddenAppearance = Appearance.Dark
            window2.overriddenAppearance = Appearance.Light
            applicationAppearance
        }

        ui {
            assertEquals(applicationAppearanceBefore, Application.appearance)
            assertEquals(Appearance.Dark, window1.overriddenAppearance)
            assertEquals(Appearance.Light, window2.overriddenAppearance)
            window1.overriddenAppearance = null
            window2.overriddenAppearance = null
        }

        ui {
            window1.close()
            window2.close()
        }
    }
}
