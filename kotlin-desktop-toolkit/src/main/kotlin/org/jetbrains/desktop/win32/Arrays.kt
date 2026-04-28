package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeAutoDropArray_u32
import org.jetbrains.desktop.win32.generated.NativeAutoDropArray_u8
import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.win32.generated.NativeFfiOption_AutoDropByteArray
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout.JAVA_BYTE
import java.lang.foreign.ValueLayout.JAVA_INT

internal fun ByteArray.toNative(arena: Arena): MemorySegment = let { bytes ->
    val result = NativeBorrowedArray_u8.allocate(arena)
    NativeBorrowedArray_u8.ptr(result, arena.allocateArray(JAVA_BYTE, *bytes))
    NativeBorrowedArray_u8.len(result, bytes.count().toLong())
    result
}

internal fun byteArrayFromNative(segment: MemorySegment): ByteArray {
    val ptr = NativeAutoDropArray_u8.ptr(segment)
    val len = NativeAutoDropArray_u8.len(segment)
    val result = ByteArray(len.toInt())
    MemorySegment.copy(ptr, JAVA_BYTE, 0, result, 0, result.size)
    return result
}

internal fun optionalByteArrayFromNative(segment: MemorySegment): ByteArray? {
    val value = NativeFfiOption_AutoDropByteArray.value(segment)
    val isSome = NativeFfiOption_AutoDropByteArray.is_some(segment)
    if (!isSome) {
        return null
    }
    return byteArrayFromNative(value)
}

internal fun intArrayFromNative(segment: MemorySegment): IntArray {
    val ptr = NativeAutoDropArray_u32.ptr(segment)
    val len = NativeAutoDropArray_u32.len(segment)
    val result = IntArray(len.toInt())
    MemorySegment.copy(ptr, JAVA_INT, 0, result, 0, result.size)
    return result
}
