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
    Changed,
}

public class ClipboardException(
    public val status: ClipboardStatus,
    public val nativeCode: Int,
    public val nativeMessage: String? = null,
) : RuntimeException(clipboardExceptionMessage(status, nativeCode, nativeMessage))

public class ClipboardChangedException(
    public val expectedChangeCount: UInt,
    public val actualChangeCount: UInt,
) : RuntimeException(
    "Clipboard changed while waiting to retry the read operation: " +
        "expected sequence $expectedChangeCount, actual sequence $actualChangeCount",
)

internal data class ClipboardOperation(
    val status: ClipboardStatus,
    val nativeCode: Int,
    val nativeMessage: String?,
) {
    fun throwIfFailed() {
        if (status != ClipboardStatus.Ok) {
            throw ClipboardException(status, nativeCode, nativeMessage)
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

internal fun ClipboardOperation.requireOkOrUnavailable(expectedChangeCount: UInt): Boolean {
    if (status == ClipboardStatus.Changed) {
        throw ClipboardChangedException(expectedChangeCount, nativeCode.toUInt())
    }
    return requireOkOrUnavailable()
}

internal fun checkClipboardReadOperation(segment: MemorySegment, expectedChangeCount: UInt) {
    val operation = clipboardOperationFromNative(segment)
    if (operation.status == ClipboardStatus.Changed) {
        throw ClipboardChangedException(expectedChangeCount, operation.nativeCode.toUInt())
    }
    operation.throwIfFailed()
}

internal fun clipboardOperationFromNative(segment: MemorySegment): ClipboardOperation {
    val status = when (NativeClipboardOperationResult.status(segment)) {
        desktop_win32_h.NativeClipboardStatus_Ok() -> ClipboardStatus.Ok
        desktop_win32_h.NativeClipboardStatus_Busy() -> ClipboardStatus.Busy
        desktop_win32_h.NativeClipboardStatus_FormatUnavailable() -> ClipboardStatus.FormatUnavailable
        desktop_win32_h.NativeClipboardStatus_DataTooLarge() -> ClipboardStatus.DataTooLarge
        desktop_win32_h.NativeClipboardStatus_InvalidData() -> ClipboardStatus.InvalidData
        desktop_win32_h.NativeClipboardStatus_NativeError() -> ClipboardStatus.NativeError
        desktop_win32_h.NativeClipboardStatus_Changed() -> ClipboardStatus.Changed
        else -> ClipboardStatus.NativeError
    }
    return ClipboardOperation(
        status,
        NativeClipboardOperationResult.code(segment),
        clipboardOperationMessageFromNative(segment),
    )
}

internal fun checkClipboardOperation(segment: MemorySegment) {
    clipboardOperationFromNative(segment).throwIfFailed()
}

private fun clipboardExceptionMessage(status: ClipboardStatus, nativeCode: Int, nativeMessage: String?): String {
    if (status == ClipboardStatus.Changed) {
        return "Clipboard operation failed: Changed (actual sequence ${nativeCode.toUInt()})"
    }
    val detail = if (nativeCode == 0) {
        ""
    } else {
        " (HRESULT 0x${nativeCode.toUInt().toString(16).padStart(8, '0')})"
    }
    val message = nativeMessage?.takeIf { it.isNotBlank() }?.let { ": $it" } ?: ""
    return "Clipboard operation failed: $status$detail$message"
}

private fun clipboardOperationMessageFromNative(segment: MemorySegment): String? {
    val messagePtr = NativeClipboardOperationResult.message(segment)
    if (messagePtr == MemorySegment.NULL) {
        return null
    }
    return stringFromNative(messagePtr)
}
