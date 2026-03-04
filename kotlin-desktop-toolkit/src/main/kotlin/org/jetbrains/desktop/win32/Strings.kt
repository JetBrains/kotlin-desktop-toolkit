package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeAutoDropArray_RustAllocatedStrPtr
import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_BorrowedStrPtr
import org.jetbrains.desktop.win32.generated.desktop_win32_h.NativeBorrowedStrPtr
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

internal fun listOfStringsFromNative(segment: MemorySegment): List<String> {
    val ptr = NativeAutoDropArray_RustAllocatedStrPtr.ptr(segment)
    val len = NativeAutoDropArray_RustAllocatedStrPtr.len(segment)

    return (0 until len).map { i ->
        val strPtr = ptr.getAtIndex(NativeAutoDropArray_RustAllocatedStrPtr.`ptr$layout`(), i)
        strPtr.getUtf8String(0)
    }.toList()
}

internal fun listOfStringsToNative(arena: Arena, list: List<String>): MemorySegment {
    val itemsCount = list.count().toLong()
    val itemsArray = arena.allocateArray(NativeBorrowedStrPtr, itemsCount)
    list.forEachIndexed { index, item ->
        val strPtr = arena.allocateUtf8String(item)
        itemsArray.setAtIndex(NativeBorrowedStrPtr, index.toLong(), strPtr)
    }
    val result = NativeBorrowedArray_BorrowedStrPtr.allocate(arena)
    NativeBorrowedArray_BorrowedStrPtr.ptr(result, itemsArray)
    NativeBorrowedArray_BorrowedStrPtr.len(result, itemsCount)
    return result
}
