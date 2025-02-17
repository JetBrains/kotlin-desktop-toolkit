package org.jetbrains.desktop.sample

import org.jetbrains.desktop.macos.AppMenuItem
import org.jetbrains.desktop.macos.AppMenuStructure
import org.jetbrains.desktop.macos.KeyModifiersSet
import org.jetbrains.desktop.macos.Keystroke
import org.jetbrains.desktop.macos.Logger
import java.time.LocalDate

private fun imLucky(): Boolean {
    return (System.currentTimeMillis() / 1000L) % 3 == 0L
}

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
            AppMenuItem.SubMenu(title = "Empty Submenu"),
        ),
        AppMenuItem.SubMenu(
            title = "File",
            AppMenuItem.Action("Foo", isEnabled = false),
            AppMenuItem.Separator,
            AppMenuItem.Action(
                "Bar",
                isEnabled = true,
                keystroke = Keystroke(
                    key = "x",
                    modifiers = KeyModifiersSet.create(control = true),
                ),
                perform = { Logger.info { "First callback from Kotlin!" } },
            ),
            AppMenuItem.SubMenu(title = "Empty Submenu"),
        ),
        AppMenuItem.SubMenu(
            title = "Edit",
            items = buildList {
                if ((System.currentTimeMillis() / 2000L) % 2 == 0L) {
                    add(AppMenuItem.Action("Flickering Item1", true))
                }
                add(AppMenuItem.Action("Foo", false))
                add(AppMenuItem.Separator)
                add(AppMenuItem.Action("Bar", true))
                add(AppMenuItem.SubMenu(title = "Empty Submenu"))
                if ((System.currentTimeMillis() / 3000L) % 2 == 0L) {
                    add(AppMenuItem.Action("Flickering Item2", true))
                }
            },
        ),
        AppMenuItem.SubMenu(
            title = "View",
            AppMenuItem.Action("View1", false),
            AppMenuItem.Separator,
            AppMenuItem.Action("View2", true),
            AppMenuItem.SubMenu(title = "Empty Submenu"),
        ),
        AppMenuItem.SubMenu(
            title = "Keystrokes",
            AppMenuItem.Action(
                "Item1",
                keystroke = Keystroke(key = "xy", modifiers = KeyModifiersSet.create()), // second letter is ignored
                perform = if (imLucky()) {
                    val f = { Logger.info { "Odd" } }
                    f
                } else {
                    val f = { Logger.info { "Even" } }
                    f
                },
            ),
            AppMenuItem.Action(
                "Item2",
                keystroke = Keystroke(key = "X", modifiers = KeyModifiersSet.create()),
            ), // shift modifier added because letter is capital
            AppMenuItem.Action("Item3", keystroke = Keystroke(key = "й", modifiers = KeyModifiersSet.create(option = true))),
            AppMenuItem.Action(
                "Item4",
                keystroke = Keystroke(key = "\u000d", modifiers = KeyModifiersSet.create(command = true)),
            ), // it's enter
            AppMenuItem.Action(
                "Item5",
                keystroke = if (imLucky()) Keystroke(key = "k", modifiers = KeyModifiersSet.create(shift = true)) else null,
            ),
        ),
        AppMenuItem.Action("Top level action", true),
        AppMenuItem.SubMenu(
            title = "FooBar2",
            AppMenuItem.Separator,
            AppMenuItem.Separator,
            AppMenuItem.Separator,
            AppMenuItem.Action("Foo", isEnabled = (System.currentTimeMillis() / 2000L) % 2 == 0L),
            AppMenuItem.Separator,
            AppMenuItem.Action("Bar", false),
            AppMenuItem.SubMenu(title = "Empty Submenu"),
        ),
        AppMenuItem.Separator,
        AppMenuItem.SubMenu(
            title = "Time",
            AppMenuItem.Action("Foo ${System.currentTimeMillis() % 100}", true),
            AppMenuItem.Separator,
            AppMenuItem.Action("Bar", false),
            AppMenuItem.SubMenu(title = "Empty Submenu"),
        ),
        AppMenuItem.SubMenu(
            title = "FooBar3",
            AppMenuItem.Action("Foo", true),
            AppMenuItem.Separator,
            AppMenuItem.Action("Bar", true),
            AppMenuItem.SubMenu(
                title = "Not empty submenu",
                AppMenuItem.Action("Action", false),
                AppMenuItem.Separator,
                AppMenuItem.Action("Date: ${LocalDate.now()}", true),
            ),
        ),
        AppMenuItem.SubMenu(
            title = "MyWindow",
            items = buildList {
                if ((System.currentTimeMillis() / 2000L) % 2 == 0L) {
                    add(AppMenuItem.Action("First Flickering Item", true))
                }
                add(AppMenuItem.Action("My Window Item1", true))
                add(AppMenuItem.Action("My Window Item2", true))
                add(AppMenuItem.Action("My Window Item3", true))
                if ((System.currentTimeMillis() / 2000L) % 2 == 0L) {
                    add(AppMenuItem.Action("Last Flickering Item", true))
                }
            },
            specialTag = "Window",
        ),
        AppMenuItem.SubMenu(
            title = "Help",
            AppMenuItem.Action("Help1", true),
            AppMenuItem.Action("Help2", true),
            AppMenuItem.Action("Help3", true),
        ),
    )
}
