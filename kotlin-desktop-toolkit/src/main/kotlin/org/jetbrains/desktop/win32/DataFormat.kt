package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import kotlin.use

@JvmInline
public value class DataFormat internal constructor(internal val id: Int) {
    public companion object {
        public val Text: DataFormat = DataFormat(13) // CF_UNICODETEXT
        public val FileList: DataFormat = DataFormat(15) // CF_HDROP

        public val Html: DataFormat by lazy {
            DataFormat(desktop_win32_h.clipboard_get_html_format_id())
        }

        public fun register(formatName: String): DataFormat {
            val formatId = ffiDownCall {
                Arena.ofConfined().use { arena ->
                    val namePtr = arena.allocateFrom(formatName)
                    desktop_win32_h.data_transfer_register_format(namePtr)
                }
            }
            return DataFormat(formatId)
        }
    }
}
