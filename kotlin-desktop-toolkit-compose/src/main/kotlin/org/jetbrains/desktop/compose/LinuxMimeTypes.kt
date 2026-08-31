package org.jetbrains.desktop.compose

import java.net.URI
import java.nio.file.Path

/**
 * The MIME types this layer speaks, and the encoding of each format's bytes.
 *
 * Where macOS has one uniform type identifier per format, the freedesktop world has an ordered list of
 * MIME types per format: the first is the one to advertise, the rest are the aliases older applications
 * offer. Reading accepts any of them, writing offers all of them.
 *
 * Shared by both Linux backends: the MIME types are the desktop's, not Wayland's or GTK's, so the
 * Wayland and GTK clipboards speak exactly the same ones.
 */
internal object LinuxMimeTypes {
    const val UTF8_PLAIN_TEXT = "text/plain;charset=utf-8"
    const val PLAIN_TEXT = "text/plain"
    const val UTF8_HTML = "text/html;charset=utf-8"
    const val HTML = "text/html"
    const val FILE_URI_LIST = "text/uri-list"
    const val PNG = "image/png"
}

internal fun ClipboardFormat<*>.linuxMimeTypes(): List<String> = when (this) {
    ClipboardFormat.Utf8PlainText -> listOf(LinuxMimeTypes.UTF8_PLAIN_TEXT, LinuxMimeTypes.PLAIN_TEXT)
    ClipboardFormat.Html -> listOf(LinuxMimeTypes.UTF8_HTML, LinuxMimeTypes.HTML)
    ClipboardFormat.Png -> listOf(LinuxMimeTypes.PNG)
    ClipboardFormat.File -> listOf(LinuxMimeTypes.FILE_URI_LIST)
}

/** Every MIME type the entry can be handed out as, in the order the formats were added. */
internal fun ClipboardItemsEntry.linuxMimeTypes(): List<String> {
    val mimeTypes = linkedSetOf<String>()
    for (item in items) {
        for (element in item.elements) {
            mimeTypes.addAll(element.format.linuxMimeTypes())
        }
    }
    return mimeTypes.toList()
}

/**
 * Encodes the entry's data for a MIME type the other side asked for, or `null` when it holds nothing that
 * can answer.
 *
 * A `text/uri-list` answer is assembled from every file element at once, because the format is one
 * newline-separated list rather than one entry per item; the single-valued formats return the first
 * element that matches.
 */
internal fun ClipboardItemsEntry.dataForLinuxMimeType(mimeType: String): ByteArray? {
    val filePaths = mutableListOf<String>()
    for (item in items) {
        for (element in item.elements) {
            if (mimeType !in element.format.linuxMimeTypes()) continue
            when (element.format) {
                ClipboardFormat.Utf8PlainText,
                ClipboardFormat.Html,
                -> return (element.value as String).toByteArray(Charsets.UTF_8)
                ClipboardFormat.Png -> return element.value as ByteArray
                ClipboardFormat.File -> filePaths.add(element.value as String)
            }
        }
    }
    if (filePaths.isEmpty()) {
        return null
    }
    // The format wants CRLF-separated file:// URIs, with a trailing separator.
    return filePaths
        .joinToString(separator = "\r\n", postfix = "\r\n") { Path.of(it).toUri().toString() }
        .toByteArray(Charsets.UTF_8)
}

internal fun <T : Any> decodeLinuxMimeData(data: ByteArray, format: ClipboardFormat<T>): List<T> {
    val decoded: List<Any> = when (format) {
        ClipboardFormat.Utf8PlainText -> listOf(data.decodeToString())
        ClipboardFormat.Html -> listOf(data.decodeToString())
        ClipboardFormat.Png -> listOf(data)
        ClipboardFormat.File -> decodeFileUriList(data)
    }
    @Suppress("UNCHECKED_CAST")
    return decoded as List<T>
}

private fun decodeFileUriList(data: ByteArray): List<String> {
    return data.decodeToString()
        .lineSequence()
        .map(String::trim)
        // The format allows comment lines, and the trailing separator leaves an empty one.
        .filter { it.isNotEmpty() && !it.startsWith("#") }
        .mapNotNull { uri ->
            runCatching { URI(uri) }.getOrNull()
                ?.takeIf { it.scheme == "file" }
                ?.path
        }
        .toList()
}
