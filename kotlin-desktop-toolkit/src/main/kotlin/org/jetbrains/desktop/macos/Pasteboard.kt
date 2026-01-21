package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_CombinedItemElement
import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_PasteboardItem
import org.jetbrains.desktop.macos.generated.NativeCombinedItemElement
import org.jetbrains.desktop.macos.generated.NativePasteboardContentResult
import org.jetbrains.desktop.macos.generated.NativePasteboardItem
import org.jetbrains.desktop.macos.generated.NativePasteboardItemDataResult
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.nio.file.Path

public object Pasteboard {
    public const val STRING_TYPE: String = "public.utf8-plain-text"
    public const val HTML_TYPE: String = "public.html"
    public const val URL_TYPE: String = "public.url"
    public const val FILE_URL_TYPE: String = "public.file-url"
    public const val PNG_IMAGE_TYPE: String = "public.png"
    public const val TIFF_IMAGE_TYPE: String = "public.tiff"

    public class Element(
        public val type: String,
        public val content: ByteArray,
    ) {
        public companion object {
            public fun ofString(type: String, content: String): Element {
                return Element(type, content.encodeToByteArray())
            }

            public fun ofFilePath(path: Path): Element {
                val urlString =
                    UrlUtils.filePathToFileReferenceUrl(path.toAbsolutePath().toString())
                        ?: error("File should exist to be placed in clipboard $path")
                return Element(FILE_URL_TYPE, urlString.encodeToByteArray())
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

    // Pasteboard writing API:

    public fun clear(pasteboard: PasteboardType = PasteboardType.General): Long {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.pasteboard_clear(
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                )
            }
        }
    }

    public fun writeObjects(vararg items: Item, pasteboard: PasteboardType = PasteboardType.General): Boolean {
        return writeObjects(items.toList(), pasteboard)
    }

    public fun writeObjects(items: List<Item>, pasteboard: PasteboardType = PasteboardType.General): Boolean {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.pasteboard_write_objects(
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                    items.toNative(arena),
                )
            }
        }
    }

    // Pasteboard reading API:

    public fun readItemsOfType(type: String, pasteboard: PasteboardType = PasteboardType.General): List<ByteArray> {
        return Arena.ofConfined().use { arena ->
            val nativeResult = ffiDownCall {
                desktop_macos_h.pasteboard_read_items_of_type(
                    arena,
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
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

    public fun readFileItemPaths(pasteboard: PasteboardType = PasteboardType.General): List<Path> {
        return readItemsOfType(FILE_URL_TYPE, pasteboard).mapNotNull { bytes ->
            UrlUtils.urlToFilePath(String(bytes))?.let { Path.of(it) }
        }
    }

    // Low-level pasteboard reading API:

    public fun changeCount(pasteboard: PasteboardType = PasteboardType.General): Long {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.pasteboard_read_change_count(
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                )
            }
        }
    }

    public fun itemCount(pasteboard: PasteboardType = PasteboardType.General): Long {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.pasteboard_read_items_count(
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                )
            }
        }
    }

    public fun readItemTypes(itemIndex: Int, pasteboard: PasteboardType = PasteboardType.General): List<String> {
        return Arena.ofConfined().use { arena ->
            val nativeResult = ffiDownCall {
                desktop_macos_h.pasteboard_read_item_types(
                    arena,
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                    itemIndex.toLong(),
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

    /**
     * Returns the data for a specific item at the given index and type.
     * Returns null if the item doesn't have data for the given type.
     */
    public fun readItemData(itemIndex: Int, type: String, pasteboard: PasteboardType = PasteboardType.General): ByteArray? {
        return Arena.ofConfined().use { arena ->
            val nativeResult = ffiDownCall {
                desktop_macos_h.pasteboard_read_item_data(
                    arena,
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                    itemIndex.toLong(),
                    arena.allocateUtf8String(type),
                )
            }
            val found = NativePasteboardItemDataResult.found(nativeResult)
            val result = if (found) {
                byteArrayFromNative(NativePasteboardItemDataResult.data(nativeResult))
            } else {
                null
            }
            ffiDownCall {
                desktop_macos_h.pasteboard_item_data_drop(nativeResult)
            }
            result
        }
    }
}

@JvmInline
public value class PasteboardType internal constructor(internal val name: String?) {
    public companion object {
        public val General: PasteboardType = PasteboardType(null)
        public fun named(name: String): PasteboardType = PasteboardType(name)
    }

    internal fun toNameOrNull(): String? = name
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
