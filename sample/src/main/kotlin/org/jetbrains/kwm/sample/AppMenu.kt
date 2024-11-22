package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.*
import java.time.LocalDate

fun buildAppMenu(): AppMenuStructure {
    /**
     * Constraints:
     * 1. Only submenus are allowed on the top level. Other items will remain in menu structure but will be ignored
     * 2. The first element of the root menu has always an application name
     * 3. We can register Windows, Help and Services menus, and OS will add some additional items there
     * 4. Menu with name Help will have search field as a first item
     * 5. Edit submenu have `AutoFill`, `Start Dictation` and Emoji & Symbols items
     * 6. `Edit` submenu needs some careful threatment, additional items might be easily removed
     * 7. `Edit` submenu items might be removed when reconciled with empty list
     * 8. Hiden items are used by macOS to handle shortcut aliases
     * 9. Multiple separators are rendered as single separator, but still remains in app menu
     * 10. View submenu may have some additional items, including `Toggle Fullscreen` and some other
     *
     * see: https://stackoverflow.com/questions/21369736/remove-start-dictation-and-special-characters-from-menu
     *      https://stackoverflow.com/questions/6391053/qt-mac-remove-special-characters-action-in-edit-menu
     *
     */
    return AppMenuStructure(
        AppMenuItem.SubMenu(
            title = "App", // Ignored
            AppMenuItem.Action("App menu item1", false),
            AppMenuItem.SubMenu("Services", specialTag = "Services"),
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
            AppMenuItem.Action("Foo", false),
            AppMenuItem.Separator,
            AppMenuItem.Action("Bar", true),
            AppMenuItem.SubMenu(title = "Empty Submenu"),
            AppMenuItem.Action("Emoji & Symbols", isMacOSProvided = true),
            specialTag = "Edit",
        ),
        AppMenuItem.SubMenu(
            title = "View",
//            AppMenuItem.Action("View1", false),
//            AppMenuItem.Separator,
//            AppMenuItem.Action("View2", true),
//            AppMenuItem.SubMenu(title = "Empty Submenu"),
//            AppMenuItem.Action("Enter Full Screen", isMacOSProvided = true),
//            AppMenuItem.Action("Exit Full Screen", isMacOSProvided = true),
            specialTag = "View",
        ),
        AppMenuItem.Action("Top level action", true),
        AppMenuItem.SubMenu(
            title = "FooBar2",
            AppMenuItem.Separator,
            AppMenuItem.Separator,
            AppMenuItem.Separator,
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
        AppMenuItem.SubMenu(
            title = "MyWindow",
            specialTag = "Window"),
        AppMenuItem.SubMenu(title = "Help",
                            AppMenuItem.Action("Help1", true),
                            AppMenuItem.Action("Help2", true),
                            AppMenuItem.Action("Help3", true))
    )
}
