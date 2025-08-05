package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.AppMenuItem
import org.jetbrains.desktop.macos.AppMenuManager
import org.jetbrains.desktop.macos.AppMenuStructure
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import kotlin.test.Test

@EnabledOnOs(OS.MAC)
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
