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

/**
 * Provides access to macOS pasteboard (clipboard) functionality.
 *
 * The pasteboard supports multiple items, each containing multiple representations (elements)
 * of different types (UTIs - Uniform Type Identifiers).
 *
 * @see <a href="https://github.com/sindresorhus/Pasteboard-Viewer">Pasteboard Viewer</a> - useful tool for debugging
 */
public object Pasteboard {
    /** UTI for plain UTF-8 text content. */
    public const val STRING_TYPE: String = "public.utf8-plain-text"
    /** UTI for HTML content. */
    public const val HTML_TYPE: String = "public.html"
    /** UTI for URL content. */
    public const val URL_TYPE: String = "public.url"
    /** UTI for file URL content (local file references). */
    public const val FILE_URL_TYPE: String = "public.file-url"
    /** UTI for PNG image data. */
    public const val PNG_IMAGE_TYPE: String = "public.png"
    /** UTI for TIFF image data. */
    public const val TIFF_IMAGE_TYPE: String = "public.tiff"

    /**
     * A single data representation within a pasteboard item.
     *
     * @property type The UTI (Uniform Type Identifier) for this element.
     * @property content The raw byte data for this element.
     */
    public class Element(
        public val type: String,
        public val content: ByteArray,
    ) {
        public companion object {
            /**
             * Creates an element from a string, encoding it as UTF-8 bytes.
             *
             * @param type The UTI for this element.
             * @param content The string content.
             * @return A new Element with the encoded content.
             */
            public fun ofString(type: String, content: String): Element {
                return Element(type, content.encodeToByteArray())
            }

            /**
             * Creates a file URL element from a file path.
             *
             * @param path The file path (must exist on the filesystem).
             * @return A new Element with [FILE_URL_TYPE] containing the file reference URL.
             * @throws IllegalStateException if the file does not exist.
             */
            public fun ofFilePath(path: Path): Element {
                val urlString =
                    UrlUtils.filePathToFileReferenceUrl(path.toAbsolutePath().toString())
                        ?: error("File should exist to be placed in clipboard $path")
                return Element(FILE_URL_TYPE, urlString.encodeToByteArray())
            }
        }
    }

    /**
     * A pasteboard item containing one or more data representations (elements).
     *
     * Multiple elements allow providing the same content in different formats,
     * enabling receivers to choose their preferred format.
     *
     * @property elements The list of data representations for this item.
     */
    public data class Item(val elements: List<Element>) {
        public constructor(vararg elements: Element) : this(elements.toList())

        public companion object {
            /**
             * Creates an item with a single string element.
             *
             * @param type The UTI for the element.
             * @param content The string content.
             * @return A new Item containing one Element.
             */
            public fun ofString(type: String, content: String): Item {
                return Item(Element(type, content.encodeToByteArray()))
            }

            /**
             * Creates an item with a single element from raw bytes.
             *
             * @param type The UTI for the element.
             * @param content The raw byte content.
             * @return A new Item containing one Element.
             */
            public fun of(type: String, content: ByteArray): Item {
                return Item(Element(type, content))
            }
        }
    }

    // Pasteboard writing API:

    /**
     * Clears all content from the pasteboard.
     *
     * @param pasteboard The pasteboard to clear. Defaults to [PasteboardType.General].
     * @return The new change count after clearing.
     */
    public fun clear(pasteboard: PasteboardType = PasteboardType.General): Long {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.pasteboard_clear(
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                )
            }
        }
    }

    /**
     * Writes items to the pasteboard, replacing existing content.
     *
     * @param items The items to write.
     * @param pasteboard The target pasteboard. Defaults to [PasteboardType.General].
     * @return `true` if the write succeeded.
     */
    public fun writeObjects(vararg items: Item, pasteboard: PasteboardType = PasteboardType.General): Boolean {
        return writeObjects(items.toList(), pasteboard)
    }

    /**
     * Writes items to the pasteboard, replacing existing content.
     *
     * @param items The list of items to write.
     * @param pasteboard The target pasteboard. Defaults to [PasteboardType.General].
     * @return `true` if the write succeeded.
     */
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

    /**
     * Reads all items of a specific type from the pasteboard.
     *
     * @param type The UTI to read.
     * @param pasteboard The source pasteboard. Defaults to [PasteboardType.General].
     * @return List of byte arrays, one per item that contains the requested type.
     */
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

    /**
     * Reads file paths from file URL items on the pasteboard.
     *
     * @param pasteboard The source pasteboard. Defaults to [PasteboardType.General].
     * @return List of file paths extracted from file URL items.
     */
    public fun readFileItemPaths(pasteboard: PasteboardType = PasteboardType.General): List<Path> {
        return readItemsOfType(FILE_URL_TYPE, pasteboard).mapNotNull { bytes ->
            UrlUtils.urlToFilePath(String(bytes))?.let { Path.of(it) }
        }
    }

    // Low-level pasteboard reading API:
    // Usually [readItemsOfType] is enough, but these methods allow analyzing all available items.
    // Note: pasteboard content may be overwritten by another application at any time.
    // Store the [changeCount] value before reading and verify it after each call.
    // If it changes, the content is stale and you should retry or abort.

    /**
     * Returns the current change count of the pasteboard.
     *
     * The change count increments each time the pasteboard content changes.
     * Useful for detecting external modifications.
     *
     * @param pasteboard The pasteboard to query. Defaults to [PasteboardType.General].
     * @return The current change count.
     */
    public fun changeCount(pasteboard: PasteboardType = PasteboardType.General): Long {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.pasteboard_read_change_count(
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                )
            }
        }
    }

    /**
     * Returns the number of items on the pasteboard.
     *
     * @param pasteboard The pasteboard to query. Defaults to [PasteboardType.General].
     * @return The number of items.
     */
    public fun itemCount(pasteboard: PasteboardType = PasteboardType.General): Long {
        return Arena.ofConfined().use { arena ->
            ffiDownCall {
                desktop_macos_h.pasteboard_read_items_count(
                    pasteboard.toNameOrNull()?.let { arena.allocateUtf8String(it) } ?: MemorySegment.NULL,
                )
            }
        }
    }

    /**
     * Returns the available UTIs for a specific item.
     *
     * If the [itemIndex] is out of bounds, returns an empty list.
     *
     * @param itemIndex The zero-based index of the item.
     * @param pasteboard The pasteboard to query. Defaults to [PasteboardType.General].
     * @return List of UTI strings available for the item.
     */
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
     *
     * Use [readItemTypes] to discover available types for an item before calling this method.
     *
     * If the [itemIndex] is out of bounds, returns `null`.
     *
     * @param itemIndex The zero-based index of the item.
     * @param type The UTI of the data to retrieve.
     * @param pasteboard The pasteboard to query. Defaults to [PasteboardType.General].
     * @return The raw byte data, or `null` if the item doesn't have data for the given type
     *         or if the index is out of bounds.
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

/**
 * Represents a pasteboard type (named pasteboard or the general pasteboard).
 *
 * macOS supports multiple named pasteboards for different purposes.
 * Use [General] for the standard system clipboard.
 */
@JvmInline
public value class PasteboardType internal constructor(internal val name: String?) {
    public companion object {
        /** The general (system) pasteboard, used for standard copy/paste. */
        public val General: PasteboardType = PasteboardType(null)

        /**
         * Creates a reference to a named pasteboard.
         *
         * @param name The pasteboard name (e.g., "com.apple.pasteboard.find").
         * @return A PasteboardType for the named pasteboard.
         */
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
