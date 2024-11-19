package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.AppMenuItem
import org.jetbrains.kwm.macos.AppMenuManager
import org.jetbrains.kwm.macos.AppMenuStructure
import org.jetbrains.kwm.macos.GrandCentralDispatch
import java.awt.Dimension
import java.awt.Point
import java.time.LocalDate
import javax.swing.JFrame

fun buildAppMenu(): AppMenuStructure {
    /**
     * Constraints:
     * 1. Only submenues area allowed on the top level.
     * 2. The first element of root menu has always an application name
     * 3. We can register Windows, Help and Services menues, and OS will add some additional items there
     * 4. Menu with name Help will have search field as a first item
     * 5. Edit submenu *sometimes* have `AutoFill`, `Start Dictation` and Emoji & Symbols items
     *
     * see: https://stackoverflow.com/questions/21369736/remove-start-dictation-and-special-characters-from-menu
     *      https://stackoverflow.com/questions/6391053/qt-mac-remove-special-characters-action-in-edit-menu
     *
     * Also some quirks might be brought by JVM and AWT
     */
    return AppMenuStructure(
        AppMenuItem.SubMenu(
            title = "App", // Ignored
            AppMenuItem.Action("App menu item1", false),
            AppMenuItem.Separator,
            AppMenuItem.Action("App menu item2", true),
            AppMenuItem.SubMenu(title = "Empty Submenu")
        ),
        AppMenuItem.SubMenu(
            title = "File",
            AppMenuItem.Action("Foo", false),
            AppMenuItem.Separator,
            AppMenuItem.Action("Bar", true),
            AppMenuItem.SubMenu(title = "Empty Submenu")
        ),
        AppMenuItem.SubMenu(
            title = "Edit",
//            AppMenuItem.Action("Foo", false),
//            AppMenuItem.Separator,
//            AppMenuItem.Action("Bar", true),
//            AppMenuItem.SubMenu(title = "Empty Submenu")
        ),
        AppMenuItem.Action("Top level action", true),
        AppMenuItem.SubMenu(
            title = "FooBar2",
            AppMenuItem.Action("Foo", true),
            AppMenuItem.Separator,
            AppMenuItem.Action("Bar", false),
            AppMenuItem.SubMenu(title = "Empty Submenu")
        ),
        AppMenuItem.Separator,
        AppMenuItem.SubMenu(
            title = "Time: ${System.currentTimeMillis() % 100}",
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
            AppMenuItem.SubMenu(title = "Not empty submenu",
                                AppMenuItem.Action("Action", false),
                                AppMenuItem.Separator,
                                AppMenuItem.Action("Date: ${LocalDate.now()}", true))
        ),
        AppMenuItem.SubMenu(title = "Window"),
        AppMenuItem.SubMenu(title = "Help",
                            AppMenuItem.Action("Help1", true),
                            AppMenuItem.Action("Help2", true),
                            AppMenuItem.Action("Help3", true))
    )
}

fun main() {
    JFrame().apply {
        size = Dimension(800, 600)
        location = Point(200, 200)
        isVisible = true
    }
    while (true) {
        GrandCentralDispatch.dispatchOnMain {
            AppMenuManager.setMainMenu(buildAppMenu())
        }
        Thread.sleep(1000)
    }
}