package org.jetbrains.desktop.tests

import org.jetbrains.desktop.macos.Appearance
import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.Window
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class AppearanceTest {
    @Test
    fun smokeTest() {
        KotlinDesktopToolkit.init()
        val window1 = GrandCentralDispatch.dispatchOnMainSync {
            Window.create(title = "Window1")
        }
        val window2 = GrandCentralDispatch.dispatchOnMainSync {
            Window.create(title = "Window2")
        }
        val applicationAppearanceBefore = GrandCentralDispatch.dispatchOnMainSync {
            val applicationAppearance = Application.appearance
            assertTrue { Appearance.entries.toTypedArray().contains(applicationAppearance) }
            assertNull(window1.overriddenAppearance)
            assertNull(window2.overriddenAppearance)
            window1.overriddenAppearance = Appearance.Dark
            window2.overriddenAppearance = Appearance.Light
            applicationAppearance
        }

        GrandCentralDispatch.dispatchOnMainSync {
            assertEquals(applicationAppearanceBefore, Application.appearance)
            assertEquals(Appearance.Dark, window1.overriddenAppearance)
            assertEquals(Appearance.Light, window2.overriddenAppearance)
            window1.overriddenAppearance = null
            window2.overriddenAppearance = null
        }

        GrandCentralDispatch.dispatchOnMainSync {
            window1.close()
            window2.close()
            Application.stopEventLoop()
        }
    }
}
