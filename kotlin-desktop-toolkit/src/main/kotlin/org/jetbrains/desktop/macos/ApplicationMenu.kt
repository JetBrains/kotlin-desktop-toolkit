package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeAppMenuItem
import org.jetbrains.desktop.macos.generated.NativeAppMenuItemCallback
import org.jetbrains.desktop.macos.generated.NativeAppMenuItem_NativeActionItem_Body
import org.jetbrains.desktop.macos.generated.NativeAppMenuItem_NativeSubMenuItem_Body
import org.jetbrains.desktop.macos.generated.NativeAppMenuKeystroke
import org.jetbrains.desktop.macos.generated.NativeAppMenuStructure
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

/**
 * Be aware capital letter turns shift modifier on
 */
public data class Keystroke(
    val key: String,
    val modifiers: KeyModifiersSet,
)

public enum class Trigger {
    KEYSTROKE,
    OTHER,
    ;

    public companion object {
        internal fun fromNative(x: Int): Trigger {
            return when (x) {
                desktop_macos_h.NativeAppMenuTrigger_Keystroke() -> KEYSTROKE
                desktop_macos_h.NativeAppMenuTrigger_Other() -> OTHER
                else -> error("Unknown trigger: $x")
            }
        }
    }
}

public sealed class AppMenuItem {
    public data class Action(
        val title: String,
        val isEnabled: Boolean = true,
        val state: ActionItemState = ActionItemState.Off,
        val keystroke: Keystroke? = null,
        val specialTag: SpecialTag = SpecialTag.None,
        val perform: (Trigger) -> Unit = {},
    ) : AppMenuItem() {
        public enum class SpecialTag {
            None,
            Undo,
            Redo,
            Cut,
            Copy,
            Paste,
            Delete,
            ;

            internal fun toNative(): Int {
                return when (this) {
                    None -> desktop_macos_h.NativeActionMenuItemSpecialTag_None()
                    Undo -> desktop_macos_h.NativeActionMenuItemSpecialTag_Undo()
                    Redo -> desktop_macos_h.NativeActionMenuItemSpecialTag_Redo()
                    Cut -> desktop_macos_h.NativeActionMenuItemSpecialTag_Cut()
                    Copy -> desktop_macos_h.NativeActionMenuItemSpecialTag_Copy()
                    Paste -> desktop_macos_h.NativeActionMenuItemSpecialTag_Paste()
                    Delete -> desktop_macos_h.NativeActionMenuItemSpecialTag_Delete()
                }
            }
        }
    }

    public enum class ActionItemState {
        // Draw check mark
        On,

        // Draw nothing
        Off,

        // Draw minus sign
        Mixed,
        ;

        internal fun toNative(): Int {
            return when (this) {
                On -> desktop_macos_h.NativeActionItemState_On()
                Off -> desktop_macos_h.NativeActionItemState_Off()
                Mixed -> desktop_macos_h.NativeActionItemState_Mixed()
            }
        }
    }

    public data object Separator : AppMenuItem()

    public class SubMenu(
        public val title: String,
        public val items: List<AppMenuItem>,
        public val specialTag: SpecialTag = SpecialTag.None,
    ) : AppMenuItem() {
        public enum class SpecialTag {
            None,
            AppNameMenu,
            Window,
            Services,
            ;

            internal fun toNative(): Int {
                return when (this) {
                    None -> desktop_macos_h.NativeSubMenuItemSpecialTag_None()
                    AppNameMenu -> desktop_macos_h.NativeSubMenuItemSpecialTag_AppNameMenu()
                    Window -> desktop_macos_h.NativeSubMenuItemSpecialTag_Window()
                    Services -> desktop_macos_h.NativeSubMenuItemSpecialTag_Services()
                }
            }
        }

        public constructor(title: String, vararg items: AppMenuItem, specialTag: SpecialTag = SpecialTag.None) : this(
            title,
            items.toList(),
            specialTag,
        )
    }
}

public data class AppMenuStructure(val items: List<AppMenuItem>) {
    public constructor(vararg items: AppMenuItem) : this(items.toList())
}

public object AppMenuManager {
    internal var callbacksArena: Arena? = null

    public fun setMainMenu(menu: AppMenuStructure) {
        ffiDownCall {
            val previousCallbackArena = callbacksArena
            callbacksArena = Arena.ofConfined()
            Arena.ofConfined().use { arena ->
                desktop_macos_h.main_menu_update(menu.toNative(arena))
            }
            previousCallbackArena?.close()
        }
    }

    public fun setMainMenuToNone() {
        ffiDownCall {
            desktop_macos_h.main_menu_set_none()
        }
    }

    public fun offerCurrentEvent() {
        ffiDownCall {
            desktop_macos_h.main_menu_offer_current_event()
        }
    }
}

private fun Keystroke.toNative(arena: Arena): MemorySegment = let { keystroke ->
    val result = NativeAppMenuKeystroke.allocate(arena)
    NativeAppMenuKeystroke.key(result, arena.allocateUtf8String(keystroke.key))
    NativeAppMenuKeystroke.modifiers(result, keystroke.modifiers.value)
    result
}

private fun AppMenuItem.toNative(nativeItem: MemorySegment, arena: Arena): Unit = let { menuItem ->
    when (menuItem) {
        is AppMenuItem.Action -> {
            NativeAppMenuItem.tag(nativeItem, desktop_macos_h.NativeAppMenuItem_ActionItem())

            val actionItemBody = NativeAppMenuItem_NativeActionItem_Body.allocate(arena)
            NativeAppMenuItem_NativeActionItem_Body.enabled(actionItemBody, menuItem.isEnabled)
            NativeAppMenuItem_NativeActionItem_Body.state(actionItemBody, menuItem.state.toNative())
            NativeAppMenuItem_NativeActionItem_Body.title(actionItemBody, arena.allocateUtf8String(menuItem.title))
            NativeAppMenuItem_NativeActionItem_Body.special_tag(actionItemBody, menuItem.specialTag.toNative())
            NativeAppMenuItem_NativeActionItem_Body.keystroke(actionItemBody, menuItem.keystroke?.toNative(arena) ?: MemorySegment.NULL)
            NativeAppMenuItem_NativeActionItem_Body.perform(
                actionItemBody,
                NativeAppMenuItemCallback.allocate({ trigger ->
                    ffiUpCall {
                        menuItem.perform(Trigger.fromNative(trigger))
                    }
                }, AppMenuManager.callbacksArena),
            )
            NativeAppMenuItem.action_item(nativeItem, actionItemBody)
        }

        is AppMenuItem.Separator -> {
            NativeAppMenuItem.tag(nativeItem, desktop_macos_h.NativeAppMenuItem_SeparatorItem())
        }

        is AppMenuItem.SubMenu -> {
            NativeAppMenuItem.tag(nativeItem, desktop_macos_h.NativeAppMenuItem_SubMenuItem())

            val itemsArray = NativeAppMenuItem.allocateArray(menuItem.items.size.toLong(), arena)
            menuItem.items.forEachIndexed { i, subMenuItem ->
                val subItemNative = NativeAppMenuItem.asSlice(itemsArray, i.toLong())
                subMenuItem.toNative(subItemNative, arena)
            }

            val subMenuItemBody = NativeAppMenuItem_NativeSubMenuItem_Body.allocate(arena)
            NativeAppMenuItem_NativeSubMenuItem_Body.title(subMenuItemBody, arena.allocateUtf8String(menuItem.title))
            NativeAppMenuItem_NativeSubMenuItem_Body.special_tag(subMenuItemBody, menuItem.specialTag.toNative())
            NativeAppMenuItem_NativeSubMenuItem_Body.items_count(subMenuItemBody, menuItem.items.size.toLong())
            NativeAppMenuItem_NativeSubMenuItem_Body.items(subMenuItemBody, itemsArray)

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
