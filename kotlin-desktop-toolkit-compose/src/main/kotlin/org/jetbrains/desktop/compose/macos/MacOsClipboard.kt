@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.compose.macos

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.Clipboard
import kotlinx.coroutines.withContext
import org.jetbrains.desktop.compose.ClipboardEntry
import org.jetbrains.desktop.compose.ClipboardFormat
import org.jetbrains.desktop.compose.ClipboardItem
import org.jetbrains.desktop.compose.ClipboardItemsEntry
import org.jetbrains.desktop.compose.KdtImmediateDispatcher
import org.jetbrains.desktop.macos.Pasteboard
import org.jetbrains.desktop.macos.PasteboardType
import java.nio.file.Path

internal object MacOsClipboard : Clipboard {

    override suspend fun getClipEntry(): ClipEntry? = ClipEntry(MacOsClipboardEntry(PasteboardType.General))

    override suspend fun setClipEntry(clipEntry: ClipEntry?) {
        val entry = clipEntry?.nativeClipEntry ?: return
        require(entry is ClipboardItemsEntry)
        withContext(KdtImmediateDispatcher) {
            Pasteboard.clear()
            Pasteboard.writeObjects(entry.items.toPasteboardItems())
        }
    }

    override val nativeClipboard: Any
        get() = Pasteboard
}

internal fun List<ClipboardItem>.toPasteboardItems(): List<Pasteboard.Item> {
    return flatMap { item ->
        val elements = item.elements.flatMap {
            @Suppress("UNCHECKED_CAST")
            when (it.format) {
                ClipboardFormat.Utf8PlainText -> listOf(
                    Pasteboard.Element.ofString(
                        UniformTypeIdentifiers.UTF8_PLAIN_TEXT,
                        it.value as String,
                    ),
                )
                ClipboardFormat.Html -> listOf(
                    Pasteboard.Element.ofString(
                        UniformTypeIdentifiers.HTML,
                        it.value as String,
                    ),
                )
                ClipboardFormat.File -> listOf(
                    Pasteboard.Element.ofFilePath(Path.of(it.value as String)),
                )
                ClipboardFormat.Png -> listOf(
                    Pasteboard.Element(UniformTypeIdentifiers.PNG, it.value as ByteArray),
                )
            }
        }

        listOf(Pasteboard.Item(elements))
    }
}

internal class MacOsClipboardEntry(private val pasteboardType: PasteboardType) : ClipboardEntry {
    override suspend fun <T : Any> getForFormat(format: ClipboardFormat<T>): List<T> {
        return withContext(KdtImmediateDispatcher) {
            getForFormatSync(format)
        }
    }

    override fun <T : Any> getForFormatSync(format: ClipboardFormat<T>): List<T> {
        return decodeClipboardData(
            format,
            readBytesForType = { uti -> Pasteboard.readItemsOfType(uti, pasteboardType) },
            readFilePaths = { Pasteboard.readFileItemPaths(pasteboardType).map { it.toString() } },
        )
    }
}

private fun <T : Any> decodeClipboardData(
    format: ClipboardFormat<T>,
    readBytesForType: (uti: String) -> List<ByteArray>,
    readFilePaths: () -> List<String>,
): List<T> {
    @Suppress("UNCHECKED_CAST")
    return when (format) {
        ClipboardFormat.Utf8PlainText -> readBytesForType(UniformTypeIdentifiers.UTF8_PLAIN_TEXT)
            .map { String(it, charset = Charsets.UTF_8) }
        ClipboardFormat.Html -> readBytesForType(UniformTypeIdentifiers.HTML)
            .map { String(it, charset = Charsets.UTF_8) }
        ClipboardFormat.Png -> readBytesForType(UniformTypeIdentifiers.PNG)
        ClipboardFormat.File -> readFilePaths()
    } as List<T>
}

internal object UniformTypeIdentifiers {
    const val UTF8_PLAIN_TEXT = Pasteboard.STRING_TYPE
    const val HTML = Pasteboard.HTML_TYPE
    const val FILE = Pasteboard.FILE_URL_TYPE
    const val PNG = Pasteboard.PNG_IMAGE_TYPE
}

internal fun <T : Any> ClipboardFormat<T>.toUniformTypeIdentifier(): String {
    return when (this) {
        ClipboardFormat.Utf8PlainText -> UniformTypeIdentifiers.UTF8_PLAIN_TEXT
        ClipboardFormat.Html -> UniformTypeIdentifiers.HTML
        ClipboardFormat.File -> UniformTypeIdentifiers.FILE
        ClipboardFormat.Png -> UniformTypeIdentifiers.PNG
    }
}
