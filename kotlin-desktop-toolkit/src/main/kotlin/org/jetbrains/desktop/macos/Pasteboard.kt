package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_CombinedItemElement
import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_PasteboardItem
import org.jetbrains.desktop.macos.generated.NativeCombinedItemElement
import org.jetbrains.desktop.macos.generated.NativePasteboardItem
import org.jetbrains.desktop.macos.generated.NativePasteboardItem_NativeCombinedItem_Body
import org.jetbrains.desktop.macos.generated.NativePasteboardItem_NativeURLItem_Body
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object Pasteboard {
    public const val STRING_TYPE: String = "public.utf8-plain-text"
    public const val HTML_TYPE: String = "public.html"

    public data class Element(val type: String, val content: String)

    public sealed class Item {
        public data class Url(val url: String) : Item()
        public data class Combined(val elements: List<Element>) : Item() {
            public constructor(vararg elements: Element) : this(elements.toList())
        }

        public companion object {
            public fun of(type: String, content: String): Item {
                return Combined(Element(type, content))
            }
        }
    }

    public fun clear(): Long {
        return ffiDownCall { desktop_macos_h.pasteboard_clear() }
    }

    /**
     * Order plays role here, items at the beginning have preference over others
     */
    public fun writeObjects(vararg items: Item): Boolean {
        return writeObjects(items.toList())
    }

    public fun writeObjects(items: List<Item>): Boolean {
        return Arena.ofConfined().use { arena ->
            ffiDownCall { desktop_macos_h.pasteboard_write_objects(items.toNative(arena)) }
        }
    }
}

// IMPL:

internal fun Pasteboard.Element.toNative(natiiveElement: MemorySegment, arena: Arena) = let { element ->
    NativeCombinedItemElement.uniform_type_identifier(natiiveElement, arena.allocateUtf8String(element.type))
    NativeCombinedItemElement.content(natiiveElement, arena.allocateUtf8String(element.content))
}

@JvmName("toNativeBorrowedArray_CombinedItemElement")
internal fun List<Pasteboard.Element>.toNative(arena: Arena): MemorySegment = let { elements ->
    val result = NativeBorrowedArray_CombinedItemElement.allocate(arena)
    val elementsArray = NativeCombinedItemElement.allocateArray(elements.count().toLong(), arena)
    elements.forEachIndexed { i, element ->
        element.toNative(NativeCombinedItemElement.asSlice(elementsArray, i.toLong()), arena)
    }
    NativeBorrowedArray_CombinedItemElement.ptr(result, elementsArray)
    NativeBorrowedArray_CombinedItemElement.len(result, elements.count().toLong())
    result
}

internal fun Pasteboard.Item.toNative(nativeItem: MemorySegment, arena: Arena) = let { item ->
    when (item) {
        is Pasteboard.Item.Url -> {
            NativePasteboardItem.tag(nativeItem, desktop_macos_h.NativePasteboardItem_URLItem())
            val body = NativePasteboardItem_NativeURLItem_Body.allocate(arena)
            NativePasteboardItem_NativeURLItem_Body.url(body, arena.allocateUtf8String(item.url))
            NativePasteboardItem.url_item(nativeItem, body)
        }
        is Pasteboard.Item.Combined -> {
            NativePasteboardItem.tag(nativeItem, desktop_macos_h.NativePasteboardItem_CombinedItem())
            val body = NativePasteboardItem_NativeCombinedItem_Body.allocate(arena)
            NativePasteboardItem_NativeCombinedItem_Body.elements(body, item.elements.toNative(arena))
            NativePasteboardItem.combined_item(nativeItem, body)
        }
    }
}

@JvmName("toNativeBorrowedArray_PasteboardItem")
internal fun List<Pasteboard.Item>.toNative(arena: Arena): MemorySegment = let { items ->
    val itemsArray = NativePasteboardItem.allocateArray(items.count().toLong(), arena)
    items.forEachIndexed { i, item ->
        item.toNative(NativePasteboardItem.asSlice(itemsArray, i.toLong()), arena)
    }

    val result = NativeBorrowedArray_PasteboardItem.allocate(arena)
    NativeBorrowedArray_PasteboardItem.ptr(result, itemsArray)
    NativeBorrowedArray_PasteboardItem.len(result, items.count().toLong())
    result
}
