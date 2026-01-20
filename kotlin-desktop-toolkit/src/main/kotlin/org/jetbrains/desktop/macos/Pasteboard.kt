package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_CombinedItemElement
import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_PasteboardItem
import org.jetbrains.desktop.macos.generated.NativeCombinedItemElement
import org.jetbrains.desktop.macos.generated.NativePasteboardContentResult
import org.jetbrains.desktop.macos.generated.NativePasteboardItem
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object Pasteboard {
    public const val STRING_TYPE: String = "public.utf8-plain-text"
    public const val HTML_TYPE: String = "public.html"
    public const val URL_TYPE: String = "public.url"
    public const val FILE_URL_TYPE: String = "public.file-url"
    public const val PNG_IMAGE_TYPE: String = "public.png"
    public const val TIFF_IMAGE_TYPE: String = "public.tiff"

    public data class Element(
        val type: String,
        val content: ByteArray,
    ) {
        public companion object {
            public fun ofString(type: String, content: String): Element {
                return Element(type, content.encodeToByteArray())
            }
        }
    }

    public data class Item(val elements: List<Element>) {
        public constructor(vararg elements: Element) : this(elements.toList())

        public companion object {
            public fun ofString(type: String, content: String): Item {
                return Item(Element(type, content.encodeToByteArray()))
            }

            public fun of(type: String, content: ByteArray): Item {
                return Item(Element(type, content))
            }
        }
    }

    public fun clear(): Long {
        return ffiDownCall { desktop_macos_h.pasteboard_clear() }
    }

    public fun changeCount(): Long {
        return ffiDownCall { desktop_macos_h.pasteboard_read_change_count() }
    }

    public fun itemCount(): Long {
        return ffiDownCall { desktop_macos_h.pasteboard_read_items_count() }
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

    /**
     * When pasteboardName is null general clipboard is used
     */
    public fun readItemsOfType(type: String, pasteboardName: String? = null): List<ByteArray> {
        return Arena.ofConfined().use { arena ->
            val nativeResult = ffiDownCall {
                desktop_macos_h.pasteboard_read_items_of_type(
                    arena,
                    pasteboardName?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                    arena.allocateUtf8String(type),
                )
            }
            val items = NativePasteboardContentResult.items(nativeResult)
            val result = listOfByteArraysFromNative(items)
            ffiDownCall {
                desktop_macos_h.pasteboard_content_drop(nativeResult)
            }
            result
        }
    }

    /**
     * When pasteboardName is null general clipboard is used
     */
    public fun readFileItemPaths(pasteboardName: String? = null): List<String> {
        return Arena.ofConfined().use { arena ->
            val nativeResult = ffiDownCall {
                desktop_macos_h.pasteboard_read_file_items(
                    arena,
                    pasteboardName?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                )
            }
            val items = NativePasteboardContentResult.items(nativeResult)
            val result = listOfByteArraysFromNative(items).map { String(it) }
            ffiDownCall {
                desktop_macos_h.pasteboard_content_drop(nativeResult)
            }
            result
        }
    }
}

// IMPL:

internal fun Pasteboard.Element.toNative(natiiveElement: MemorySegment, arena: Arena) = let { element ->
    NativeCombinedItemElement.uniform_type_identifier(natiiveElement, arena.allocateUtf8String(element.type))
    NativeCombinedItemElement.content(natiiveElement, element.content.toNative(arena))
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
    NativePasteboardItem.elements(nativeItem, item.elements.toNative(arena))
}

@JvmName("toNativeBorrowedArray_PasteboardItem")
internal fun List<Pasteboard.Item>.toNative(arena: Arena): MemorySegment = let { items ->
    val itemsCount = items.count().toLong()
    val itemsArray = NativePasteboardItem.allocateArray(itemsCount, arena)
    items.forEachIndexed { i, item ->
        item.toNative(NativePasteboardItem.asSlice(itemsArray, i.toLong()), arena)
    }

    val result = NativeBorrowedArray_PasteboardItem.allocate(arena)
    NativeBorrowedArray_PasteboardItem.ptr(result, itemsArray)
    NativeBorrowedArray_PasteboardItem.len(result, itemsCount)
    result
}
