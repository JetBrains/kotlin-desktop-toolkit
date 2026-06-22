package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeAutoDropArray_RustAllocatedStrPtr
import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_BorrowedStrPtr
import org.jetbrains.desktop.win32.generated.NativeFfiOption_AutoDropArray_RustAllocatedStrPtr
import org.jetbrains.desktop.win32.generated.NativeFfiOption_RustAllocatedStrPtr
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import org.jetbrains.desktop.win32.generated.desktop_win32_h.NativeBorrowedStrPtr
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

/**
 * Copies and drops a Rust-owned `AutoDropArray<RustAllocatedStrPtr>`.
 */
internal fun listOfStringsFromNative(segment: MemorySegment): List<String> {
    val ptr = NativeAutoDropArray_RustAllocatedStrPtr.ptr(segment)
    val len = NativeAutoDropArray_RustAllocatedStrPtr.len(segment)

    return try {
        (0 until len).map { i ->
            val strPtr = ptr.getAtIndex(NativeAutoDropArray_RustAllocatedStrPtr.`ptr$layout`(), i)
            strPtr.getString(0)
        }.toList()
    } finally {
        ffiDownCall {
            desktop_win32_h.native_string_array_drop(segment)
        }
    }
}

internal fun optionalListOfStringsFromNative(segment: MemorySegment): List<String>? {
    val isSome = NativeFfiOption_AutoDropArray_RustAllocatedStrPtr.is_some(segment)
    if (!isSome) {
        return null
    }
    val value = NativeFfiOption_AutoDropArray_RustAllocatedStrPtr.value(segment)
    val ptr = NativeAutoDropArray_RustAllocatedStrPtr.ptr(value)
    val len = NativeAutoDropArray_RustAllocatedStrPtr.len(value)

    return try {
        (0 until len).map { i ->
            val strPtr = ptr.getAtIndex(NativeAutoDropArray_RustAllocatedStrPtr.`ptr$layout`(), i)
            strPtr.getString(0)
        }.toList()
    } finally {
        ffiDownCall {
            desktop_win32_h.native_optional_string_array_drop(segment)
        }
    }
}

internal fun listOfStringsToNative(arena: Arena, list: List<String>): MemorySegment {
    val itemsCount = list.count().toLong()
    val itemsArray = arena.allocate(NativeBorrowedStrPtr, itemsCount)
    list.forEachIndexed { index, item ->
        val strPtr = arena.allocateFrom(item)
        itemsArray.setAtIndex(NativeBorrowedStrPtr, index.toLong(), strPtr)
    }
    val result = NativeBorrowedArray_BorrowedStrPtr.allocate(arena)
    NativeBorrowedArray_BorrowedStrPtr.ptr(result, itemsArray)
    NativeBorrowedArray_BorrowedStrPtr.len(result, itemsCount)
    return result
}

/**
 * Copies and drops a Rust-owned `RustAllocatedStrPtr`.
 */
internal fun stringFromNative(segment: MemorySegment): String {
    check(segment != MemorySegment.NULL) { "Native string was null" }
    return try {
        segment.getString(0)
    } finally {
        ffiDownCall {
            desktop_win32_h.native_string_drop(segment)
        }
    }
}

internal fun optionalStringFromNative(segment: MemorySegment): String? {
    val isSome = NativeFfiOption_RustAllocatedStrPtr.is_some(segment)
    if (!isSome) {
        return null
    }
    val strPtr = NativeFfiOption_RustAllocatedStrPtr.value(segment)
    return try {
        strPtr.getString(0)
    } finally {
        ffiDownCall {
            desktop_win32_h.native_optional_string_drop(segment)
        }
    }
}
