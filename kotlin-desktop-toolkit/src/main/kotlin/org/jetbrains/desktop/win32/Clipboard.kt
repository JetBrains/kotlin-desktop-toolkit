package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeTransferBoolResult
import org.jetbrains.desktop.win32.generated.NativeTransferDataObjectResult
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment

/**
 * Outcome of a [Clipboard] operation.
 *
 * The OLE clipboard can momentarily fail — most importantly, another process can hold it open
 * ([TransferStatus.Busy]) — and this API leaves retrying to the caller, so every [Clipboard]
 * call returns a result instead of throwing.
 */
public sealed interface ClipboardResult<out T> {
    public data class Success<out T>(public val value: T) : ClipboardResult<T>

    /**
     * A failed operation. [status] classifies the failure; [TransferStatus.Busy] means the
     * clipboard was locked and the same call can be retried.
     */
    public data class Failure(
        public val status: TransferStatus,
        public val nativeCode: Int,
        public val message: String?,
    ) : ClipboardResult<Nothing>
}

/** True when the operation failed because the clipboard was locked and can be retried. */
public val ClipboardResult<*>.isBusy: Boolean
    get() = this is ClipboardResult.Failure && status == TransferStatus.Busy

/**
 * Low-level, synchronous binding to the Windows OLE clipboard
 * (`OleGetClipboard` / `OleSetClipboard` / `OleFlushClipboard`).
 *
 * Each method performs exactly one OLE call and returns a [ClipboardResult]. The toolkit
 * deliberately does not layer retry scheduling, change-count guarding, or thread marshaling on
 * top: using the OLE clipboard *correctly* is the caller's responsibility.
 *
 *  - **Thread.** Every method must run on a thread with an initialized OLE STA. In practice that
 *    is the [Application] dispatcher thread — call these from an event handler or via
 *    [Application.invokeOnDispatcher]. Calling from another thread is undefined.
 *  - **Contention.** Another process can momentarily hold the clipboard open. OLE then fails with
 *    `CLIPBRD_E_CANT_OPEN`, returned here as [ClipboardResult.Failure] with
 *    [ClipboardResult.Failure.status] == [TransferStatus.Busy] (see [isBusy]). Whether and how to
 *    retry (for example with a short backoff) is up to you — and you should: see the example below.
 *  - **Concurrent changes.** To detect that the clipboard changed between a read and a later write,
 *    capture [sequenceNumber] and compare.
 *
 * ### Publishing data
 *
 * [set] uses *delayed rendering*: the clipboard holds only a pointer to your [DataObject] and the
 * data is materialized on demand, so the application must keep pumping its message loop while the
 * object is on the clipboard. OLE takes its own reference on [set], so you may [DataObject.close]
 * your handle right afterwards.
 *
 *  - To copy data that must stay available **after the application exits**, call [set] then [flush]:
 *    [flush] renders every format into the clipboard and releases OLE's reference.
 *  - To support **lazy rendering** (materialize only when something is pasted), call [set] and do
 *    not [flush]; keep the application running and use [isCurrent] to check whether your object is
 *    still the one on the clipboard.
 *
 * ```
 * // Copy text that survives application exit, retrying past transient contention.
 * application.invokeOnDispatcher {
 *     DataObject.build { addTextItem("Hello clipboard!") }.use { data ->
 *         clipboardWithRetry { Clipboard.set(data) }
 *         clipboardWithRetry { Clipboard.flush() }
 *     }
 * }
 *
 * // A caller-owned helper that retries only the "busy" case.
 * fun <T> clipboardWithRetry(attempts: Int = 8, op: () -> ClipboardResult<T>): T {
 *     repeat(attempts) {
 *         when (val result = op()) {
 *             is ClipboardResult.Success -> return result.value
 *             is ClipboardResult.Failure -> {
 *                 check(result.isBusy) { "clipboard operation failed: ${result.status}" }
 *                 Thread.sleep(10)
 *             }
 *         }
 *     }
 *     error("clipboard stayed busy")
 * }
 * ```
 */
public object Clipboard {
    /**
     * `GetClipboardSequenceNumber`: a value that changes whenever the clipboard contents change.
     * Capture it to later detect concurrent modifications. This query does not fail.
     */
    public fun sequenceNumber(): UInt {
        return ffiDownCall {
            desktop_win32_h.clipboard_get_sequence_number().toUInt()
        }
    }

    /**
     * `OleGetClipboard`: the data object currently on the clipboard. On [ClipboardResult.Success]
     * the caller owns the returned [DataObject] and must [DataObject.close] it.
     */
    public fun get(): ClipboardResult<DataObject> = Arena.ofConfined().use { arena ->
        val result = ffiDownCall {
            desktop_win32_h.clipboard_read_result(arena)
        }
        decodeClipboardResult(NativeTransferDataObjectResult.result(result)) {
            DataObject(NativeTransferDataObjectResult.value(result))
        }
    }

    /**
     * `OleSetClipboard`: places [dataObject] on the clipboard using delayed rendering. OLE takes its
     * own reference, so [dataObject] may be closed afterwards. Call [flush] to make the data persist
     * after the application exits.
     */
    public fun set(dataObject: DataObject): ClipboardResult<Unit> = Arena.ofConfined().use { arena ->
        val result = ffiDownCall {
            desktop_win32_h.clipboard_set_data_object_result(arena, dataObject.toNative())
        }
        decodeClipboardResult(result) { }
    }

    /**
     * `OleFlushClipboard`: renders the data object set with [set] into the clipboard and releases
     * OLE's reference to it, so the data remains available after the application exits.
     */
    public fun flush(): ClipboardResult<Unit> = Arena.ofConfined().use { arena ->
        val result = ffiDownCall {
            desktop_win32_h.clipboard_flush_result(arena)
        }
        decodeClipboardResult(result) { }
    }

    /**
     * `OleSetClipboard(NULL)`: empties the clipboard. Use [flush] instead when the data should
     * remain available after the application exits.
     */
    public fun clear(): ClipboardResult<Unit> = Arena.ofConfined().use { arena ->
        val result = ffiDownCall {
            desktop_win32_h.clipboard_clear_result(arena)
        }
        decodeClipboardResult(result) { }
    }

    /**
     * `OleIsCurrentClipboard`: whether [dataObject] (previously passed to [set]) is still the data
     * object on the clipboard. Only meaningful for a data object you set yourself.
     */
    public fun isCurrent(dataObject: DataObject): ClipboardResult<Boolean> = Arena.ofConfined().use { arena ->
        val result = ffiDownCall {
            desktop_win32_h.clipboard_is_current_data_object_result(arena, dataObject.toNative())
        }
        decodeClipboardResult(NativeTransferBoolResult.result(result)) {
            NativeTransferBoolResult.value(result)
        }
    }
}

private inline fun <T> decodeClipboardResult(operationSegment: MemorySegment, onSuccess: () -> T): ClipboardResult<T> {
    val operation = transferOperationFromNative(operationSegment)
    return if (operation.status == TransferStatus.Ok) {
        ClipboardResult.Success(onSuccess())
    } else {
        ClipboardResult.Failure(operation.status, operation.nativeCode, operation.nativeMessage)
    }
}
