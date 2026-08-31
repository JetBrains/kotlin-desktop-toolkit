package org.jetbrains.desktop.compose

import java.io.ByteArrayOutputStream
import java.nio.file.Path

/**
 * Decodes one path out of a file chooser's response.
 *
 * The XDG portal answers with percent-encoded `file://` URIs, which is neither what a [Path]
 * takes nor something `URI` will decode for us without also mangling the parts that are already literal.
 * `+` is deliberately left alone: unlike in form encoding, it is a literal plus in a URI path.
 *
 * Shared by both Linux backends: the portal is the desktop's, so Wayland and GTK get the same encoding.
 */
internal fun decodeFileChooserPath(encoded: String): Path = Path.of(percentDecodeUtf8(encoded.removePrefix("file://")))

private fun percentDecodeUtf8(value: String): String {
    if ('%' !in value) return value
    val bytes = ByteArrayOutputStream(value.length)
    var index = 0
    while (index < value.length) {
        val char = value[index]
        if (char == '%' && index + 2 < value.length) {
            val code = value.substring(index + 1, index + 3).toIntOrNull(16)
            if (code != null) {
                bytes.write(code)
                index += 3
                continue
            }
        }
        // A surrogate half encoded on its own would become a replacement character; keep pairs together.
        val charCount = if (Character.isHighSurrogate(char) &&
            index + 1 < value.length &&
            Character.isLowSurrogate(value[index + 1])
        ) {
            2
        } else {
            1
        }
        bytes.write(value.substring(index, index + charCount).toByteArray(Charsets.UTF_8))
        index += charCount
    }
    return String(bytes.toByteArray(), Charsets.UTF_8)
}
