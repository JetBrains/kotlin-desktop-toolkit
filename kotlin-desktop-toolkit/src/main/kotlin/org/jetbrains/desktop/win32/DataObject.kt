package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeClipboardBoolResult
import org.jetbrains.desktop.win32.generated.NativeClipboardByteArrayResult
import org.jetbrains.desktop.win32.generated.NativeClipboardStringArrayResult
import org.jetbrains.desktop.win32.generated.NativeClipboardStringResult
import org.jetbrains.desktop.win32.generated.NativeClipboardUInt32ArrayResult
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

/**
 * Dispatcher-thread-bound and not thread-safe. Query, read, retain, and close it on
 * the application's OLE STA. This contract is documented but not consistently enforced.
 */
public class DataObject(private var comInterfacePtr: MemorySegment) : AutoCloseable {
    public fun isFormatAvailable(format: DataFormat): Boolean = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_is_format_available_result(arena, ptr, format.id)
            }
            checkClipboardOperation(NativeClipboardBoolResult.result(result))
            NativeClipboardBoolResult.value(result)
        }
    }

    public fun listItemFormats(): List<DataFormat> = requireOpen { ptr ->
        val formatIds = Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_enum_formats_result(arena, ptr)
            }
            checkClipboardOperation(NativeClipboardUInt32ArrayResult.result(result))
            val formatsPtr = NativeClipboardUInt32ArrayResult.value(result)
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
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_read_bytes_result(arena, ptr, format.id)
            }
            checkClipboardOperation(NativeClipboardByteArrayResult.result(result))
            val dataPtr = NativeClipboardByteArrayResult.value(result)
            try {
                byteArrayFromNative(dataPtr)
            } finally {
                ffiDownCall {
                    desktop_win32_h.native_byte_array_drop(dataPtr)
                }
            }
        }
    }

    public fun tryReadItemOfType(format: DataFormat): ByteArray? = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_read_bytes_result(arena, ptr, format.id)
            }
            val operation = clipboardOperationFromNative(NativeClipboardByteArrayResult.result(result))
            if (!operation.requireOkOrUnavailable()) {
                return@use null
            }
            val dataPtr = NativeClipboardByteArrayResult.value(result)
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
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_read_html_fragment_result(arena, ptr)
            }
            checkClipboardOperation(NativeClipboardStringResult.result(result))
            stringFromNative(NativeClipboardStringResult.value(result))
        }
    }

    public fun tryReadHtmlFragment(): String? = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_read_html_fragment_result(arena, ptr)
            }
            val operation = clipboardOperationFromNative(NativeClipboardStringResult.result(result))
            if (!operation.requireOkOrUnavailable()) {
                return@use null
            }
            stringFromNative(NativeClipboardStringResult.value(result))
        }
    }

    public fun readListOfFiles(): List<String> = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_read_file_list_result(arena, ptr)
            }
            checkClipboardOperation(NativeClipboardStringArrayResult.result(result))
            listOfStringsFromNative(NativeClipboardStringArrayResult.value(result))
        }
    }

    public fun tryReadListOfFiles(): List<String>? = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_read_file_list_result(arena, ptr)
            }
            val operation = clipboardOperationFromNative(NativeClipboardStringArrayResult.result(result))
            if (!operation.requireOkOrUnavailable()) {
                return@use null
            }
            listOfStringsFromNative(NativeClipboardStringArrayResult.value(result))
        }
    }

    public fun readTextItem(): String = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_read_text_result(arena, ptr)
            }
            checkClipboardOperation(NativeClipboardStringResult.result(result))
            stringFromNative(NativeClipboardStringResult.value(result))
        }
    }

    public fun tryReadTextItem(): String? = requireOpen { ptr ->
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.com_data_object_read_text_result(arena, ptr)
            }
            val operation = clipboardOperationFromNative(NativeClipboardStringResult.result(result))
            if (!operation.requireOkOrUnavailable()) {
                return@use null
            }
            stringFromNative(NativeClipboardStringResult.value(result))
        }
    }

    internal fun toNative(): MemorySegment = requireOpen { it }

    internal fun retain(): DataObject = requireOpen { ptr ->
        val retainedPtr = ffiDownCall {
            desktop_win32_h.com_data_object_retain(ptr)
        }
        check(retainedPtr != MemorySegment.NULL) { "Failed to retain DataObject" }
        DataObject(retainedPtr)
    }

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
        /**
         * Must be called from the application dispatcher thread for OLE clipboard or drag-and-drop use.
         */
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
            val strPtr = arena.allocateFrom(fragment)
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
            val strPtr = arena.allocateFrom(text)
            ffiDownCall {
                desktop_win32_h.data_object_add_from_text(dataObjectId, strPtr)
            }
        }
    }
}
