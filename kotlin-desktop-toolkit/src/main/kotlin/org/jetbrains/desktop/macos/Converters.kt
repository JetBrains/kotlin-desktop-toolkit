package org.jetbrains.desktop.macos

import org.jetbrains.desktop.LogicalPoint
import org.jetbrains.desktop.LogicalSize
import org.jetbrains.desktop.PhysicalSize
import org.jetbrains.desktop.macos.generated.NativeColor
import org.jetbrains.desktop.macos.generated.NativeLogicalPoint
import org.jetbrains.desktop.macos.generated.NativeLogicalSize
import org.jetbrains.desktop.macos.generated.NativePhysicalSize
import org.jetbrains.desktop.macos.generated.NativeSetMarkedTextOperation
import org.jetbrains.desktop.macos.generated.NativeTextChangedOperation
import org.jetbrains.desktop.macos.generated.NativeTextCommandOperation
import org.jetbrains.desktop.macos.generated.NativeTextOperation
import org.jetbrains.desktop.macos.generated.NativeUnmarkTextOperation
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

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

internal fun TextOperation.Companion.fromNative(s: MemorySegment): TextOperation {
    return when (NativeTextOperation.tag(s)) {
        desktop_macos_h.NativeTextOperation_TextChanged() -> {
            val nativeEvent = NativeTextOperation.text_changed(s)
            TextOperation.TextChanged(
                windowId = NativeTextChangedOperation.window_id(nativeEvent),
                text = NativeTextChangedOperation.text(nativeEvent).getUtf8String(0),
            )
        }
        desktop_macos_h.NativeTextOperation_TextCommand() -> {
            val nativeEvent = NativeTextOperation.text_command(s)
            TextOperation.TextCommand(
                windowId = NativeTextCommandOperation.window_id(nativeEvent),
                command = NativeTextCommandOperation.command(nativeEvent).getUtf8String(0),
            )
        }
        desktop_macos_h.NativeTextOperation_UnmarkText() -> {
            val nativeEvent = NativeTextOperation.unmark_text(s)
            TextOperation.UnmarkText(
                windowId = NativeUnmarkTextOperation.window_id(nativeEvent),
            )
        }
        desktop_macos_h.NativeTextOperation_SetMarkedText() -> {
            val nativeEvent = NativeTextOperation.set_marked_text(s)
            TextOperation.SetMarkedText(
                windowId = NativeSetMarkedTextOperation.window_id(nativeEvent),
            )
        }
        else -> {
            error("Unexpected TextOperation tag")
        }
    }
}
