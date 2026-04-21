package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class DataObject(private var comInterfacePtr: MemorySegment) : AutoCloseable {
    public fun isFormatAvailable(format: DataFormat): Boolean = requireOpen { ptr ->
        ffiDownCall {
            desktop_win32_h.com_data_object_is_format_available(ptr, format.id)
        }
    }

    public fun listItemFormats(): List<DataFormat> = requireOpen { ptr ->
        val formatIds = Arena.ofConfined().use { arena ->
            val formatsPtr = ffiDownCall {
                desktop_win32_h.com_data_object_enum_formats(arena, ptr)
            }
            try {
                intArrayFromNative(formatsPtr)
            } finally {
                ffiDownCall {
                    desktop_win32_h.native_u32_array_drop(formatsPtr)
                }
            }
        }
        formatIds.map(DataFormat::fromNative)
    }

    public fun readItemOfType(format: DataFormat): ByteArray = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val dataPtr = ffiDownCall {
                desktop_win32_h.com_data_object_read_bytes(arena, ptr, format.id)
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

    public fun readHtmlFragment(): String = requireOpen { ptr ->
        val strPtr = ffiDownCall {
            desktop_win32_h.com_data_object_read_html_fragment(ptr)
        }
        stringFromNative(strPtr)
    }

    public fun readListOfFiles(): List<String> = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val arrayPtr = ffiDownCall {
                desktop_win32_h.com_data_object_read_file_list(arena, ptr)
            }
            listOfStringsFromNative(arrayPtr)
        }
    }

    public fun readTextItem(): String = requireOpen { ptr ->
        val strPtr = ffiDownCall {
            desktop_win32_h.com_data_object_read_text(ptr)
        }
        stringFromNative(strPtr)
    }

    internal fun toNative(): MemorySegment = requireOpen { it }

    override fun close() {
        if (comInterfacePtr != MemorySegment.NULL) {
            ffiDownCall {
                desktop_win32_h.com_data_object_release(comInterfacePtr)
            }
            comInterfacePtr = MemorySegment.NULL
        }
    }

    private inline fun <R> requireOpen(block: (MemorySegment) -> R): R {
        val ptr = comInterfacePtr
        check(ptr != MemorySegment.NULL) { "DataObject has been closed" }
        return block(ptr)
    }

    public companion object {
        public fun build(block: DataObjectBuilder.() -> Unit): DataObject {
            val dataObjectId = ffiDownCall {
                desktop_win32_h.data_object_create()
            }
            val builder = DataObjectBuilder(dataObjectId)
            try {
                builder.block()
            } catch (e: Throwable) {
                ffiDownCall {
                    desktop_win32_h.data_object_drop(dataObjectId)
                }
                throw e
            }
            val ptr = ffiDownCall {
                desktop_win32_h.data_object_into_com(dataObjectId)
            }
            return DataObject(ptr)
        }
    }
}

public class DataObjectBuilder internal constructor(private val dataObjectId: Long) {
    public fun addItemOfType(format: DataFormat, data: ByteArray): Boolean {
        return Arena.ofConfined().use { arena ->
            val dataPtr = data.toNative(arena)
            ffiDownCall {
                desktop_win32_h.data_object_add_from_bytes(dataObjectId, format.id, dataPtr)
            }
        }
    }

    public fun addHtmlFragment(fragment: String): Boolean {
        return Arena.ofConfined().use { arena ->
            val strPtr = arena.allocateUtf8String(fragment)
            ffiDownCall {
                desktop_win32_h.data_object_add_from_html_fragment(dataObjectId, strPtr)
            }
        }
    }

    public fun addListOfFiles(fileNames: List<String>): Boolean {
        return Arena.ofConfined().use { arena ->
            val dataPtr = listOfStringsToNative(arena, fileNames)
            ffiDownCall {
                desktop_win32_h.data_object_add_from_file_list(dataObjectId, dataPtr)
            }
        }
    }

    public fun addTextItem(text: String): Boolean {
        return Arena.ofConfined().use { arena ->
            val strPtr = arena.allocateUtf8String(text)
            ffiDownCall {
                desktop_win32_h.data_object_add_from_text(dataObjectId, strPtr)
            }
        }
    }
}
