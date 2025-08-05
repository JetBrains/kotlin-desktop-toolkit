package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.AppMenuItem
import org.jetbrains.desktop.macos.AppMenuManager
import org.jetbrains.desktop.macos.AppMenuStructure
import kotlin.test.Test

class ApplicationMenuTest : KDTApplicationTestBase() {
    @Test
    fun smokeTest() {
        ui {
            AppMenuManager.setMainMenu(AppMenuStructure())
        }

        ui {
            AppMenuManager.setMainMenu(
                AppMenuStructure(
                    AppMenuItem.Action("Foo", false),
                    AppMenuItem.Separator,
                    AppMenuItem.Action("Bar", true),
                    AppMenuItem.SubMenu(
                        title = "FooBar",
                        AppMenuItem.Action("Foo", false),
                        AppMenuItem.Separator,
                        AppMenuItem.Action("Bar", true),
                        AppMenuItem.SubMenu(title = "Empty Submenu"),
                    ),
                ),
            )
        }
    }
}
