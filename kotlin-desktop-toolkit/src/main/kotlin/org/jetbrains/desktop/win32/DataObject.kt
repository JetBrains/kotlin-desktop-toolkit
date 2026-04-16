package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

public class DataObject(private var ptr: MemorySegment) : AutoCloseable {
    public fun readBytes(format: DataFormat): ByteArray {
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

    override fun close() {
        if (ptr != MemorySegment.NULL) {
            ffiDownCall {
                desktop_win32_h.data_object_release(ptr)
            }
            ptr = MemorySegment.NULL
        }
    }
}
