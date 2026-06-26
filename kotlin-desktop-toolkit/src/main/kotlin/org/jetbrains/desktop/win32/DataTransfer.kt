package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeDataTransferOperationResult
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.MemorySegment

/** Classification of a native data-transfer operation outcome. */
public enum class DataTransferStatus {
    Ok,
    Busy,
    FormatUnavailable,
    DataTooLarge,
    InvalidData,
    NativeError,
}

/**
 * Thrown by the throwing [DataObject] read accessors (`read*`). A [DataObject] may originate from
 * the clipboard or from a drag-and-drop drop, so this is a data-transfer failure rather than a
 * clipboard-specific one. The non-throwing [Clipboard] surface returns [ClipboardResult] instead.
 */
public class DataTransferException(
    public val status: DataTransferStatus,
    public val nativeCode: Int,
    public val nativeMessage: String? = null,
) : RuntimeException(dataTransferExceptionMessage(status, nativeCode, nativeMessage))

internal data class DataTransferOperation(
    val status: DataTransferStatus,
    val nativeCode: Int,
    val nativeMessage: String?,
) {
    fun throwIfFailed() {
        if (status != DataTransferStatus.Ok) {
            throw DataTransferException(status, nativeCode, nativeMessage)
        }
    }

    /**
     * Returns false if the requested format is unavailable, throws [DataTransferException]
     * for any other failure, and returns true on success.
     */
    fun requireOkOrUnavailable(): Boolean {
        if (status == DataTransferStatus.FormatUnavailable) {
            return false
        }
        throwIfFailed()
        return true
    }
}

internal fun dataTransferOperationFromNative(segment: MemorySegment): DataTransferOperation {
    val status = when (NativeDataTransferOperationResult.status(segment)) {
        desktop_win32_h.NativeDataTransferStatus_Ok() -> DataTransferStatus.Ok
        desktop_win32_h.NativeDataTransferStatus_Busy() -> DataTransferStatus.Busy
        desktop_win32_h.NativeDataTransferStatus_FormatUnavailable() -> DataTransferStatus.FormatUnavailable
        desktop_win32_h.NativeDataTransferStatus_DataTooLarge() -> DataTransferStatus.DataTooLarge
        desktop_win32_h.NativeDataTransferStatus_InvalidData() -> DataTransferStatus.InvalidData
        desktop_win32_h.NativeDataTransferStatus_NativeError() -> DataTransferStatus.NativeError
        else -> DataTransferStatus.NativeError
    }
    return DataTransferOperation(
        status,
        NativeDataTransferOperationResult.code(segment),
        dataTransferOperationMessageFromNative(segment),
    )
}

internal fun checkDataTransferOperation(segment: MemorySegment) {
    dataTransferOperationFromNative(segment).throwIfFailed()
}

private fun dataTransferExceptionMessage(status: DataTransferStatus, nativeCode: Int, nativeMessage: String?): String {
    val detail = if (nativeCode == 0) {
        ""
    } else {
        " (HRESULT 0x${nativeCode.toHexString()})"
    }
    val message = nativeMessage?.takeIf { it.isNotBlank() }?.let { ": $it" } ?: ""
    return "Data transfer failed: $status$detail$message"
}

private fun dataTransferOperationMessageFromNative(segment: MemorySegment): String? {
    val messagePtr = NativeDataTransferOperationResult.message(segment)
    if (messagePtr == MemorySegment.NULL) {
        return null
    }
    return stringFromNative(messagePtr)
}
