package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.*
import java.time.LocalDate

fun buildAppMenu(): AppMenuStructure {
    /**
     * Constraints:
     * 1. Only submenues are allowed on the top level
     * 2. The first element of the root menu has always an application name
     * 3. We can register Windows, Help and Services menues, and OS will add some additional items there
     * 4. Menu with name Help will have search field as a first item
     * 5. Edit submenu *sometimes* have `AutoFill`, `Start Dictation` and Emoji & Symbols items
     * 6. `Edit` submenu needs some careful threatment
     * 7. `Edit` submenu items might be removed when reconciled with empty list
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
            title = "Time",
            AppMenuItem.Action("Foo ${System.currentTimeMillis() % 100}", true),
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
