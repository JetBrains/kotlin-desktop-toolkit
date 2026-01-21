package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

/**
 * macOS URL conversion API.
 *
 * Provides utilities for converting between file paths and various URL formats.
 */
public object UrlUtils {
    /**
     * Converts a file path to a file URL string.
     *
     * @param filePath The file system path (e.g., "/Users/name/file.txt")
     * @return The file URL (e.g., "file:///Users/name/file.txt"), or null if conversion fails
     */
    public fun filePathToFileUrl(filePath: String): String? {
        return Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_macos_h.url_file_path_to_file_url(arena.allocateUtf8String(filePath))
            }
            stringFromNullableNativePtr(result)
        }
    }

    /**
     * Converts a URL string to a file path.
     *
     * Works with both regular file URLs (file://) and file reference URLs (file:///.file/id=).
     *
     * @param url The URL string (e.g., "file:///Users/name/file.txt" or "file:///.file/id=...")
     * @return The file system path (e.g., "/Users/name/file.txt"), or null if the URL cannot be interpreted as a file path
     */
    public fun urlToFilePath(url: String): String? {
        return Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_macos_h.url_to_file_path(arena.allocateUtf8String(url))
            }
            stringFromNullableNativePtr(result)
        }
    }

    /**
     * Converts a file path to a file reference URL string.
     *
     * File reference URLs use a unique file system identifier that persists across
     * file renames and moves within the same volume. This is useful for tracking
     * files even if they are renamed or moved.
     *
     * @param filePath The file system path (file must exist)
     * @return The file reference URL (e.g., "file:///.file/id=..."), or null if the file doesn't exist
     * @see <a href="https://developer.apple.com/documentation/foundation/nsurl/filereferenceurl()">Apple Documentation</a>
     */
    public fun filePathToFileReferenceUrl(filePath: String): String? {
        return Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_macos_h.url_file_path_to_file_reference_url(arena.allocateUtf8String(filePath))
            }
            stringFromNullableNativePtr(result)
        }
    }

    private fun stringFromNullableNativePtr(ptr: MemorySegment): String? {
        if (ptr == MemorySegment.NULL) return null
        val str = ptr.getUtf8String(0)
        ffiDownCall { desktop_macos_h.string_drop(ptr) }
        return str
    }
}
