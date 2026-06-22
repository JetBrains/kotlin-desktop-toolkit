package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeClipboardOperationResult
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.MemorySegment

public enum class ClipboardStatus {
    Ok,
    Busy,
    FormatUnavailable,
    DataTooLarge,
    InvalidData,
    NativeError,
}

public class ClipboardException(
    public val status: ClipboardStatus,
    public val nativeCode: Int,
) : RuntimeException(clipboardExceptionMessage(status, nativeCode))

internal data class ClipboardOperation(
    val status: ClipboardStatus,
    val nativeCode: Int,
) {
    fun throwIfFailed() {
        if (status != ClipboardStatus.Ok) {
            throw ClipboardException(status, nativeCode)
        }
    }

    /**
     * Returns false if the requested format is unavailable, throws [ClipboardException]
     * for any other failure, and returns true on success.
     */
    fun requireOkOrUnavailable(): Boolean {
        if (status == ClipboardStatus.FormatUnavailable) {
            return false
        }
        throwIfFailed()
        return true
    }
}

internal fun clipboardOperationFromNative(segment: MemorySegment): ClipboardOperation {
    val status = when (NativeClipboardOperationResult.status(segment)) {
        desktop_win32_h.NativeClipboardStatus_Ok() -> ClipboardStatus.Ok
        desktop_win32_h.NativeClipboardStatus_Busy() -> ClipboardStatus.Busy
        desktop_win32_h.NativeClipboardStatus_FormatUnavailable() -> ClipboardStatus.FormatUnavailable
        desktop_win32_h.NativeClipboardStatus_DataTooLarge() -> ClipboardStatus.DataTooLarge
        desktop_win32_h.NativeClipboardStatus_InvalidData() -> ClipboardStatus.InvalidData
        desktop_win32_h.NativeClipboardStatus_NativeError() -> ClipboardStatus.NativeError
        else -> ClipboardStatus.NativeError
    }
    return ClipboardOperation(status, NativeClipboardOperationResult.code(segment))
}

internal fun checkClipboardOperation(segment: MemorySegment) {
    clipboardOperationFromNative(segment).throwIfFailed()
}

private fun clipboardExceptionMessage(status: ClipboardStatus, nativeCode: Int): String {
    val detail = if (nativeCode == 0) {
        ""
    } else {
        " (HRESULT 0x${nativeCode.toUInt().toString(16).padStart(8, '0')})"
    }
    return "Clipboard operation failed: $status$detail"
}
