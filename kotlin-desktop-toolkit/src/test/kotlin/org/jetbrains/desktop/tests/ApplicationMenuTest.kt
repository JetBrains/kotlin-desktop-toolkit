package org.jetbrains.desktop.tests

import org.jetbrains.desktop.macos.AppMenuItem
import org.jetbrains.desktop.macos.AppMenuStructure
import org.jetbrains.desktop.macos.AppMenuManager
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import kotlin.test.Test

class ApplicationMenuTest {
    @Test
    fun smokeTest() {
        KotlinDesktopToolkit.init()
        GrandCentralDispatch.dispatchOnMain {
            AppMenuManager.setMainMenu(AppMenuStructure())
            AppMenuManager.setMainMenu(AppMenuStructure(
                AppMenuItem.Action("Foo", false),
                AppMenuItem.Separator,
                AppMenuItem.Action("Bar", true),
                AppMenuItem.SubMenu(
                    title = "FooBar",
                    AppMenuItem.Action("Foo", false),
                    AppMenuItem.Separator,
                    AppMenuItem.Action("Bar", true),
                    AppMenuItem.SubMenu(title = "Empty Submenu")
                )))
        }
        GrandCentralDispatch.dispatchOnMainSync {}
    }
}