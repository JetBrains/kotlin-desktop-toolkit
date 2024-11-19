package org.jetbrains.kwm.sample

import org.jetbrains.kwm.Library
import org.jetbrains.kwm.macos.AppMenuItem
import org.jetbrains.kwm.macos.AppMenuManager
import org.jetbrains.kwm.macos.AppMenuStructure
import org.jetbrains.kwm.macos.GrandCentralDispatch
import java.awt.Dimension
import java.awt.Point
import javax.swing.JFrame

fun main() {
    JFrame().apply {
        size = Dimension(800, 600)
        location = Point(200, 200)
        isVisible = true
    }
    GrandCentralDispatch.dispatchOnMain {
        AppMenuManager.setMainMenu(AppMenuStructure(
            AppMenuItem.SubMenu(
                title = "FooBar1",
                AppMenuItem.Action("Foo", false),
                AppMenuItem.Separator,
                AppMenuItem.Action("Bar", true),
                AppMenuItem.SubMenu(title = "Empty Submenu")
            ),
            AppMenuItem.SubMenu(
                title = "FooBar2",
                AppMenuItem.Action("Foo", true),
                AppMenuItem.Separator,
                AppMenuItem.Action("Bar", false),
                AppMenuItem.SubMenu(title = "Empty Submenu")
            ),
            AppMenuItem.SubMenu(
                title = "FooBar3",
                AppMenuItem.Action("Foo", true),
                AppMenuItem.Separator,
                AppMenuItem.Action("Bar", true),
                AppMenuItem.SubMenu(title = "Empty Submenu")
            )))
    }
}