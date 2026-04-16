package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class DataObject(private var ptr: MemorySegment) : AutoCloseable {
    public companion object {
        public fun createForItemOfType(format: DataFormat, data: ByteArray): DataObject {
            val ptr = ffiDownCall {
                Arena.ofConfined().use { arena ->
                    val dataPtr = data.toNative(arena)
                    desktop_win32_h.data_object_create_from_bytes(format.id, dataPtr)
                }
            }
            return DataObject(ptr)
        }

        public fun createForHtmlFragment(fragment: String): DataObject {
            val ptr = ffiDownCall {
                Arena.ofConfined().use { arena ->
                    val strPtr = arena.allocateUtf8String(fragment)
                    desktop_win32_h.data_object_create_from_html_fragment(strPtr)
                }
            }
            return DataObject(ptr)
        }

        public fun createForListOfFiles(fileNames: List<String>): DataObject {
            val ptr = ffiDownCall {
                Arena.ofConfined().use { arena ->
                    val dataPtr = listOfStringsToNative(arena, fileNames)
                    desktop_win32_h.data_object_create_from_file_list(dataPtr)
                }
            }
            return DataObject(ptr)
        }

        public fun createForTextItem(text: String): DataObject {
            val ptr = ffiDownCall {
                Arena.ofConfined().use { arena ->
                    val strPtr = arena.allocateUtf8String(text)
                    desktop_win32_h.data_object_create_from_text(strPtr)
                }
            }
            return DataObject(ptr)
        }
    }

    public fun isFormatAvailable(format: DataFormat): Boolean {
        return ffiDownCall {
            desktop_win32_h.clipboard_is_format_available(ptr, format.id)
        }
    }

    public fun readItemOfType(format: DataFormat): ByteArray {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                val dataPtr = desktop_win32_h.data_object_read_bytes(arena, ptr, format.id)
                try {
                    byteArrayFromNative(dataPtr)
                } finally {
                    desktop_win32_h.native_byte_array_drop(dataPtr)
                }
            }
        }
    }

    public fun readHtmlFragment(): String {
        return ffiDownCall {
            val strPtr = desktop_win32_h.data_object_read_html_fragment(ptr)
            stringFromNative(strPtr)
        }
    }

    public fun readListOfFiles(): List<String> {
        return ffiDownCall {
            Arena.ofConfined().use { arena ->
                val arrayPtr = desktop_win32_h.data_object_read_file_list(arena, ptr)
                listOfStringsFromNative(arrayPtr)
            }
        }
    }

    public fun readTextItem(): String {
        return ffiDownCall {
            val strPtr = desktop_win32_h.data_object_read_text(ptr)
            stringFromNative(strPtr)
        }
    }

    public fun toNative(): MemorySegment = ptr

    override fun close() {
        if (ptr != MemorySegment.NULL) {
            ffiDownCall {
                desktop_win32_h.data_object_release(ptr)
            }
            ptr = MemorySegment.NULL
        }
    }
}
