package org.jetbrains.kwm.tests

import org.jetbrains.kwm.macos.AppMenuItem
import org.jetbrains.kwm.macos.AppMenuStructure
import org.jetbrains.kwm.macos.AppMenuManager
import org.jetbrains.kwm.macos.GrandCentralDispatch
import kotlin.test.Test

class ApplicationMenuTest {
    @Test
    fun smokeTest() {
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