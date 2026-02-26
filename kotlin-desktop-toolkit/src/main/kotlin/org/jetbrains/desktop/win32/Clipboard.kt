package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena

public object Clipboard {
    public fun empty(owner: Window) {
        ffiDownCall {
            owner.withPointer { windowPtr ->
                desktop_win32_h.clipboard_empty(windowPtr)
            }
        }
    }

    public fun getData(owner: Window, format: ClipboardFormat): ByteArray {
        return ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val dataPtr = desktop_win32_h.clipboard_get_data(arena, windowPtr, format.id)
                    try {
                        byteArrayFromNative(dataPtr)
                    } finally {
                        desktop_win32_h.native_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    public fun getText(owner: Window): String {
        return ffiDownCall {
            val strPtr = owner.withPointer { windowPtr ->
                desktop_win32_h.clipboard_get_text(windowPtr)
            }
            try {
                strPtr.getUtf8String(0)
            } finally {
                desktop_win32_h.native_string_drop(strPtr)
            }
        }
    }

    public fun setData(owner: Window, format: ClipboardFormat, data: ByteArray) {
        ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val dataPtr = arena.allocate(data.size.toLong())
                    desktop_win32_h.clipboard_set_data(windowPtr, format.id, dataPtr)
                }
            }
        }
    }

    public fun setText(owner: Window, text: String) {
        ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val strPtr = arena.allocateUtf8String(text)
                    desktop_win32_h.clipboard_set_text(windowPtr, strPtr)
                }
            }
        }
    }

    public fun listAvailableFormats(owner: Window): List<ClipboardFormat> {
        val formatIds = ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val formatsPtr = desktop_win32_h.clipboard_list_formats(arena, windowPtr)
                    try {
                        intArrayFromNative(formatsPtr)
                    } finally {
                        desktop_win32_h.native_u32_array_drop(formatsPtr)
                    }
                }
            }
        }
        return formatIds.map { formatId ->
            when (formatId) {
                ClipboardFormat.ClipboardTextFormat.id -> ClipboardFormat.ClipboardTextFormat
                else -> ClipboardFormat.ClipboardCustomFormat(formatId)
            }
        }
    }
}

public sealed class ClipboardFormat(internal val id: Int) {
    public object ClipboardTextFormat : ClipboardFormat(13) // CF_UNICODETEXT
    public data class ClipboardCustomFormat(val formatId: Int) : ClipboardFormat(formatId)

    public companion object {
        public fun register(formatName: String): ClipboardFormat {
            val formatId = ffiDownCall {
                Arena.ofConfined().use { arena ->
                    val namePtr = arena.allocateUtf8String(formatName)
                    desktop_win32_h.clipboard_register_format(namePtr)
                }
            }
            return ClipboardCustomFormat(formatId)
        }
    }
}
