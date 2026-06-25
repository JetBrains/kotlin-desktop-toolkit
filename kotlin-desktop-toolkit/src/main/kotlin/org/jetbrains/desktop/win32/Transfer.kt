package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeTransferOperationResult
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.MemorySegment

/** Classification of a native data-transfer operation outcome. */
public enum class TransferStatus {
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
public class TransferException(
    public val status: TransferStatus,
    public val nativeCode: Int,
    public val nativeMessage: String? = null,
) : RuntimeException(transferExceptionMessage(status, nativeCode, nativeMessage))

internal data class TransferOperation(
    val status: TransferStatus,
    val nativeCode: Int,
    val nativeMessage: String?,
) {
    fun throwIfFailed() {
        if (status != TransferStatus.Ok) {
            throw TransferException(status, nativeCode, nativeMessage)
        }
    }

    /**
     * Returns false if the requested format is unavailable, throws [TransferException]
     * for any other failure, and returns true on success.
     */
    fun requireOkOrUnavailable(): Boolean {
        if (status == TransferStatus.FormatUnavailable) {
            return false
        }
        throwIfFailed()
        return true
    }
}

internal fun transferOperationFromNative(segment: MemorySegment): TransferOperation {
    val status = when (NativeTransferOperationResult.status(segment)) {
        desktop_win32_h.NativeTransferStatus_Ok() -> TransferStatus.Ok
        desktop_win32_h.NativeTransferStatus_Busy() -> TransferStatus.Busy
        desktop_win32_h.NativeTransferStatus_FormatUnavailable() -> TransferStatus.FormatUnavailable
        desktop_win32_h.NativeTransferStatus_DataTooLarge() -> TransferStatus.DataTooLarge
        desktop_win32_h.NativeTransferStatus_InvalidData() -> TransferStatus.InvalidData
        desktop_win32_h.NativeTransferStatus_NativeError() -> TransferStatus.NativeError
        else -> TransferStatus.NativeError
    }
    return TransferOperation(
        status,
        NativeTransferOperationResult.code(segment),
        transferOperationMessageFromNative(segment),
    )
}

internal fun checkTransferOperation(segment: MemorySegment) {
    transferOperationFromNative(segment).throwIfFailed()
}

private fun transferExceptionMessage(status: TransferStatus, nativeCode: Int, nativeMessage: String?): String {
    val detail = if (nativeCode == 0) {
        ""
    } else {
        " (HRESULT 0x${nativeCode.toUInt().toString(16).padStart(8, '0')})"
    }
    val message = nativeMessage?.takeIf { it.isNotBlank() }?.let { ": $it" } ?: ""
    return "Data transfer failed: $status$detail$message"
}

private fun transferOperationMessageFromNative(segment: MemorySegment): String? {
    val messagePtr = NativeTransferOperationResult.message(segment)
    if (messagePtr == MemorySegment.NULL) {
        return null
    }
    return stringFromNative(messagePtr)
}
