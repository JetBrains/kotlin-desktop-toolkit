package org.jetbrains.kwm.macos

import org.jetbrains.kwm.macos.generated.ActionItem_Body
import org.jetbrains.kwm.macos.generated.SubMenuItem_Body
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import org.jetbrains.kwm.macos.generated.AppMenuItem as NativeAppMenuItem
import org.jetbrains.kwm.macos.generated.AppMenuStructure as NativeAppMenuStructure
import org.jetbrains.kwm.macos.generated.AppMenuKeystroke as NativeAppMenuKeystroke
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

data class Modifiers(
    val capsLock: Boolean = false,
    val shift: Boolean = false,
    val control: Boolean = false,
    val option: Boolean = false,
    val command: Boolean = false,
    val numericPad: Boolean = false,
    val help: Boolean = false,
    val function: Boolean = false)

/**
 * Be aware capital letter turns shift modifier on
 */
data class Keystroke(val key: String, val modifiers: Modifiers)

sealed class AppMenuItem {
    data class Action(val title : String,
                      val isEnabled: Boolean = true,
                      val keystroke: Keystroke? = null,
                      val isMacOSProvided: Boolean = false,
                      val perform: () -> Unit = {}): AppMenuItem()
    data object Separator: AppMenuItem()
    class SubMenu(val title: String,
                  val items: List<AppMenuItem>,
                  val specialTag: String? = null): AppMenuItem() {
        constructor(title: String, vararg items: AppMenuItem, specialTag: String? = null) : this(title, items.toList(), specialTag)
    }
}

data class AppMenuStructure(val items: List<AppMenuItem>) {
    constructor(vararg items: AppMenuItem) : this(items.toList())
}

object AppMenuManager {
    internal var callbacksArena: Arena? = null

    fun setMainMenu(menu: AppMenuStructure) {
        val previousCallbackArena = callbacksArena
        callbacksArena = Arena.ofConfined()
        Arena.ofConfined().use { arena ->
            kwm_macos_h.main_menu_update(menu.toNative(arena))
        }
        previousCallbackArena?.close()
    }

    fun setMainMenuToNone() {
        kwm_macos_h.main_menu_set_none()
    }
}

private fun Modifiers.toNative(): Int = let { modifier ->
    var result = 0
    if (modifier.capsLock) {
        result = result or kwm_macos_h.AppMenuKeyModifiers_ModifierFlagCapsLock()
    }
    if (modifier.shift) {
        result = result or kwm_macos_h.AppMenuKeyModifiers_ModifierFlagShift()
    }
    if (modifier.control) {
        result = result or kwm_macos_h.AppMenuKeyModifiers_ModifierFlagControl()
    }
    if (modifier.option) {
        result = result or kwm_macos_h.AppMenuKeyModifiers_ModifierFlagOption()
    }
    if (modifier.command) {
        result = result or kwm_macos_h.AppMenuKeyModifiers_ModifierFlagCommand()
    }
    if (modifier.numericPad) {
        result = result or kwm_macos_h.AppMenuKeyModifiers_ModifierFlagNumericPad()
    }
    if (modifier.help) {
        result = result or kwm_macos_h.AppMenuKeyModifiers_ModifierFlagHelp()
    }
    if (modifier.function) {
        result = result or kwm_macos_h.AppMenuKeyModifiers_ModifierFlagFunction()
    }
    result
}

private fun Keystroke.toNative(arena: Arena): MemorySegment = let { keystroke ->
    val result = NativeAppMenuKeystroke.allocate(arena)
    NativeAppMenuKeystroke.key(result, arena.allocateUtf8String(keystroke.key))
    NativeAppMenuKeystroke.modifiers(result, keystroke.modifiers.toNative())
    result
}

private fun AppMenuItem.toNative(nativeItem: MemorySegment, arena: Arena): Unit = let { menuItem ->
    when (menuItem) {
        is AppMenuItem.Action -> {
            NativeAppMenuItem.tag(nativeItem, kwm_macos_h.ActionItem())

            val actionItemBody = ActionItem_Body.allocate(arena)
            ActionItem_Body.enabled(actionItemBody, menuItem.isEnabled)
            ActionItem_Body.title(actionItemBody, arena.allocateUtf8String(menuItem.title))
            ActionItem_Body.macos_provided(actionItemBody, menuItem.isMacOSProvided)
            ActionItem_Body.keystroke(actionItemBody, menuItem.keystroke?.toNative(arena) ?: MemorySegment.NULL)
            ActionItem_Body.perform(actionItemBody, ActionItem_Body.perform.allocate(menuItem.perform,
                                                                                     AppMenuManager.callbacksArena))
            NativeAppMenuItem.action_item(nativeItem, actionItemBody)
        }

        is AppMenuItem.Separator -> {
            NativeAppMenuItem.tag(nativeItem, kwm_macos_h.SeparatorItem())
        }

        is AppMenuItem.SubMenu -> {
            NativeAppMenuItem.tag(nativeItem, kwm_macos_h.SubMenuItem())

            val itemsArray = NativeAppMenuItem.allocateArray(menuItem.items.size.toLong(), arena)
            menuItem.items.forEachIndexed { i, subMenuItem ->
                val subItemNative = NativeAppMenuItem.asSlice(itemsArray, i.toLong())
                subMenuItem.toNative(subItemNative, arena)
            }

            val subMenuItemBody = SubMenuItem_Body.allocate(arena)
            SubMenuItem_Body.title(subMenuItemBody, arena.allocateUtf8String(menuItem.title))
            SubMenuItem_Body.special_tag(subMenuItemBody, menuItem.specialTag?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL)
            SubMenuItem_Body.items_count(subMenuItemBody, menuItem.items.size.toLong())
            SubMenuItem_Body.items(subMenuItemBody, itemsArray)

            NativeAppMenuItem.sub_menu_item(nativeItem, subMenuItemBody)
        }
    }
}

private fun AppMenuStructure.toNative(arena: Arena): MemorySegment = let { menuStructure ->
    val itemsCount = menuStructure.items.size.toLong()

    val itemsArray = NativeAppMenuItem.allocateArray(itemsCount, arena)
    menuStructure.items.forEachIndexed { i, menuItem ->
        menuItem.toNative(NativeAppMenuItem.asSlice(itemsArray, i.toLong()), arena)
    }

    val nativeAppMenuStructure = NativeAppMenuStructure.allocate(arena)
    NativeAppMenuStructure.items_count(nativeAppMenuStructure, itemsCount)
    NativeAppMenuStructure.items(nativeAppMenuStructure, itemsArray)

    return nativeAppMenuStructure
}