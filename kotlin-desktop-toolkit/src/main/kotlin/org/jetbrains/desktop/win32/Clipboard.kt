package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public object Clipboard {
    public fun clear(owner: Window) {
        owner.withPointer { windowPtr ->
            ffiDownCall {
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
        return owner.withPointer { windowPtr ->
            ffiDownCall {
                desktop_win32_h.clipboard_count_formats(windowPtr)
            }
        }
    }

    public fun isFormatAvailable(owner: Window, format: DataFormat): Boolean {
        return owner.withPointer { windowPtr ->
            ffiDownCall {
                desktop_win32_h.clipboard_is_format_available(windowPtr, format.id)
            }
        }
    }

    public fun listItemFormats(owner: Window): List<DataFormat> {
        val formatIds = owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val formatsPtr = ffiDownCall {
                    desktop_win32_h.clipboard_enum_formats(arena, windowPtr)
                }
                try {
                    intArrayFromNative(formatsPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_u32_array_drop(formatsPtr)
                    }
                }
            }
        }
        return formatIds.map(DataFormat::fromNative)
    }

    public fun readItemOfType(owner: Window, format: DataFormat): ByteArray {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val dataPtr = ffiDownCall {
                    desktop_win32_h.clipboard_get_data(arena, windowPtr, format.id)
                }
                try {
                    byteArrayFromNative(dataPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    public fun tryReadItemOfType(owner: Window, format: DataFormat): ByteArray? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val dataPtr = ffiDownCall {
                    desktop_win32_h.clipboard_try_get_data(arena, windowPtr, format.id)
                }
                try {
                    optionalByteArrayFromNative(dataPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_optional_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    public fun readHtmlFragment(owner: Window): String {
        val strPtr = owner.withPointer { windowPtr ->
            ffiDownCall {
                desktop_win32_h.clipboard_get_html_fragment(windowPtr)
            }
        }
        return stringFromNative(strPtr)
    }

    public fun tryReadHtmlFragment(owner: Window): String? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val optionalPtr = ffiDownCall {
                    desktop_win32_h.clipboard_try_get_html_fragment(arena, windowPtr)
                }
                optionalStringFromNative(optionalPtr)
            }
        }
    }

    public fun readListOfFiles(owner: Window): List<String> {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val arrayPtr = ffiDownCall {
                    desktop_win32_h.clipboard_get_file_list(arena, windowPtr)
                }
                listOfStringsFromNative(arrayPtr)
            }
        }
    }

    public fun tryReadListOfFiles(owner: Window): List<String>? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val arrayPtr = ffiDownCall {
                    desktop_win32_h.clipboard_try_get_file_list(arena, windowPtr)
                }
                optionalListOfStringsFromNative(arrayPtr)
            }
        }
    }

    public fun readTextItem(owner: Window): String {
        val strPtr = owner.withPointer { windowPtr ->
            ffiDownCall {
                desktop_win32_h.clipboard_get_text(windowPtr)
            }
        }
        return stringFromNative(strPtr)
    }

    public fun tryReadTextItem(owner: Window): String? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val strPtr = ffiDownCall {
                    desktop_win32_h.clipboard_try_get_text(arena, windowPtr)
                }
                optionalStringFromNative(strPtr)
            }
        }
    }

    public fun writeItemOfType(owner: Window, format: DataFormat, data: ByteArray) {
        owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val dataPtr = data.toNative(arena)
                ffiDownCall {
                    desktop_win32_h.clipboard_set_data(windowPtr, format.id, dataPtr)
                }
            }
        }
    }

    public fun writeHtmlFragment(owner: Window, fragment: String) {
        owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val strPtr = arena.allocateUtf8String(fragment)
                ffiDownCall {
                    desktop_win32_h.clipboard_set_html_fragment(windowPtr, strPtr)
                }
            }
        }
    }

    public fun writeListOfFiles(owner: Window, fileNames: List<String>) {
        owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val dataPtr = listOfStringsToNative(arena, fileNames)
                ffiDownCall {
                    desktop_win32_h.clipboard_set_file_list(windowPtr, dataPtr)
                }
            }
        }
    }

    public fun writeTextItem(owner: Window, text: String) {
        owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val strPtr = arena.allocateUtf8String(text)
                ffiDownCall {
                    desktop_win32_h.clipboard_set_text(windowPtr, strPtr)
                }
            }
        }
    }
}

public object OleClipboard {
    public fun clear() {
        ffiDownCall {
            desktop_win32_h.ole_clipboard_empty()
        }
    }

    public fun readClipboard(): DataObject {
        val ptr = ffiDownCall {
            desktop_win32_h.ole_clipboard_get_data()
        }
        check(ptr != MemorySegment.NULL) { "Failed to read from the OLE clipboard" }
        return DataObject(ptr)
    }

    public fun writeToClipboard(dataObject: DataObject) {
        ffiDownCall {
            desktop_win32_h.ole_clipboard_set_data(dataObject.toNative())
        }
    }
}
