package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.NativeAutoDropArray_AutoDropArray_u8
import org.jetbrains.desktop.macos.generated.NativeAutoDropArray_RustAllocatedStrPtr
import org.jetbrains.desktop.macos.generated.NativeAutoDropArray_u8
import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_BorrowedStrPtr
import org.jetbrains.desktop.macos.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.macos.generated.NativeColor
import org.jetbrains.desktop.macos.generated.NativeLogicalPoint
import org.jetbrains.desktop.macos.generated.NativeLogicalRect
import org.jetbrains.desktop.macos.generated.NativeLogicalSize
import org.jetbrains.desktop.macos.generated.NativePhysicalSize
import org.jetbrains.desktop.macos.generated.desktop_macos_h.NativeBorrowedStrPtr
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.lang.foreign.ValueLayout.JAVA_BYTE

internal fun LogicalSize.Companion.fromNative(s: MemorySegment): LogicalSize {
    return LogicalSize(
        width = NativeLogicalSize.width(s),
        height = NativeLogicalSize.height(s),
    )
}

internal fun LogicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalSize.allocate(arena)
    NativeLogicalSize.width(result, width)
    NativeLogicalSize.height(result, height)
    return result
}

internal fun LogicalPoint.Companion.fromNative(s: MemorySegment): LogicalPoint {
    return LogicalPoint(
        x = NativeLogicalPoint.x(s),
        y = NativeLogicalPoint.y(s),
    )
}

internal fun LogicalPoint.toNative(arena: Arena): MemorySegment {
    val result = NativeLogicalPoint.allocate(arena)
    NativeLogicalPoint.x(result, x)
    NativeLogicalPoint.y(result, y)
    return result
}

internal fun PhysicalSize.Companion.fromNative(s: MemorySegment): PhysicalSize {
    return PhysicalSize(
        width = NativePhysicalSize.width(s),
        height = NativePhysicalSize.height(s),
    )
}

internal fun PhysicalSize.toNative(arena: Arena): MemorySegment {
    val result = NativePhysicalSize.allocate(arena)
    NativePhysicalSize.width(result, width)
    NativePhysicalSize.height(result, height)
    return result
}

internal fun LogicalRect.toNative(s: MemorySegment) {
    NativeLogicalPoint.x(NativeLogicalRect.origin(s), origin.x)
    NativeLogicalPoint.y(NativeLogicalRect.origin(s), origin.y)
    NativeLogicalSize.width(NativeLogicalRect.size(s), size.width)
    NativeLogicalSize.height(NativeLogicalRect.size(s), size.height)
}

// internal fun PhysicalPoint.Companion.fromNative(s: MemorySegment): PhysicalPoint {
//    return PhysicalPoint(x = NativePhysicalPoint.x(s),
//                 y = NativePhysicalPoint.y(s))
// }

internal fun Color.toNative(arena: Arena): MemorySegment {
    val result = NativeColor.allocate(arena)
    NativeColor.red(result, red)
    NativeColor.green(result, green)
    NativeColor.blue(result, blue)
    NativeColor.alpha(result, alpha)
    return result
}

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

internal fun listOfByteArraysFromNative(segment: MemorySegment): List<ByteArray> {
    val ptr = NativeAutoDropArray_AutoDropArray_u8.ptr(segment)
    val len = NativeAutoDropArray_AutoDropArray_u8.len(segment)
    return (0 until len).map { i ->
        val byteArrayPtr = NativeAutoDropArray_u8.asSlice(ptr, i)
        byteArrayFromNative(byteArrayPtr)
    }.toList()
}
