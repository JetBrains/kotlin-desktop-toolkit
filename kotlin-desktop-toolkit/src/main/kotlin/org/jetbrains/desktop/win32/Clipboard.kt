package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena

public object Clipboard {
    public fun clear(owner: Window) {
        ffiDownCall {
            owner.withPointer { windowPtr ->
                desktop_win32_h.clipboard_empty(windowPtr)
            }
        }
    }

    public fun changeCount(): UInt {
        return ffiDownCall {
            desktop_win32_h.clipboard_get_sequence_number().toUInt()
        }
    }

    public fun itemCount(owner: Window): Int {
        return ffiDownCall {
            owner.withPointer { windowPtr ->
                desktop_win32_h.clipboard_count_formats(windowPtr)
            }
        }
    }

    public fun isFormatAvailable(owner: Window, format: ClipboardFormat): Boolean {
        return ffiDownCall {
            owner.withPointer { windowPtr ->
                desktop_win32_h.clipboard_is_format_available(windowPtr, format.id)
            }
        }
    }

    public fun listItemFormats(owner: Window): List<ClipboardFormat> {
        val formatIds = ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val formatsPtr = desktop_win32_h.clipboard_enum_formats(arena, windowPtr)
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
                ClipboardFormat.Text.id -> ClipboardFormat.Text
                else -> ClipboardFormat(formatId)
            }
        }
    }

    public fun readItemOfType(owner: Window, format: ClipboardFormat): ByteArray {
        return ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val dataPtr = desktop_win32_h.clipboard_get_data(arena, windowPtr, format.id)
                    try {
                        byteArrayFromNative(dataPtr) ?: ByteArray(0)
                    } finally {
                        desktop_win32_h.native_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    public fun readHtmlFragment(owner: Window): String {
        return ffiDownCall {
            val strPtr = owner.withPointer { windowPtr ->
                desktop_win32_h.clipboard_get_html_fragment(windowPtr)
            }
            try {
                strPtr.getUtf8String(0)
            } finally {
                desktop_win32_h.native_string_drop(strPtr)
            }
        }
    }

    public fun readListOfFiles(owner: Window): List<String> {
        return ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val arrayPtr = desktop_win32_h.clipboard_get_file_list(arena, windowPtr)
                    try {
                        listOfStringsFromNative(arrayPtr)
                    } finally {
                        desktop_win32_h.native_string_array_drop(arrayPtr)
                    }
                }
            }
        }
    }

    public fun readTextItem(owner: Window): String {
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

    public fun tryReadItemOfType(owner: Window, format: ClipboardFormat): ByteArray? {
        return ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val dataPtr = desktop_win32_h.clipboard_try_get_data(arena, windowPtr, format.id)
                    try {
                        byteArrayFromNative(dataPtr)
                    } finally {
                        desktop_win32_h.native_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    public fun writeItemOfType(owner: Window, format: ClipboardFormat, data: ByteArray) {
        ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val dataPtr = data.toNative(arena)
                    desktop_win32_h.clipboard_set_data(windowPtr, format.id, dataPtr)
                }
            }
        }
    }

    public fun writeHtmlFragment(owner: Window, fragment: String) {
        ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val strPtr = arena.allocateUtf8String(fragment)
                    desktop_win32_h.clipboard_set_html_fragment(windowPtr, strPtr)
                }
            }
        }
    }

    public fun writeListOfFiles(owner: Window, fileNames: List<String>) {
        ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val dataPtr = listOfStringsToNative(arena, fileNames)
                    desktop_win32_h.clipboard_set_file_list(windowPtr, dataPtr)
                }
            }
        }
    }

    public fun writeTextItem(owner: Window, text: String) {
        ffiDownCall {
            owner.withPointer { windowPtr ->
                Arena.ofConfined().use { arena ->
                    val strPtr = arena.allocateUtf8String(text)
                    desktop_win32_h.clipboard_set_text(windowPtr, strPtr)
                }
            }
        }
    }
}

@JvmInline
public value class ClipboardFormat internal constructor(internal val id: Int) {
    public companion object {
        public val Text: ClipboardFormat = ClipboardFormat(13) // CF_UNICODETEXT
        public val FileList: ClipboardFormat = ClipboardFormat(15) // CF_HDROP

        public val Html: ClipboardFormat by lazy {
            ClipboardFormat(desktop_win32_h.clipboard_get_html_format_id())
        }

        public fun register(formatName: String): ClipboardFormat {
            val formatId = ffiDownCall {
                Arena.ofConfined().use { arena ->
                    val namePtr = arena.allocateUtf8String(formatName)
                    desktop_win32_h.clipboard_register_format(namePtr)
                }
            }
            return ClipboardFormat(formatId)
        }
    }
}
