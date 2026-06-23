package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_BorrowedStrPtr
import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_ClipboardSetItem
import org.jetbrains.desktop.win32.generated.NativeBorrowedArray_u8
import org.jetbrains.desktop.win32.generated.NativeClipboardBoolResult
import org.jetbrains.desktop.win32.generated.NativeClipboardByteArrayResult
import org.jetbrains.desktop.win32.generated.NativeClipboardDataObjectResult
import org.jetbrains.desktop.win32.generated.NativeClipboardIntResult
import org.jetbrains.desktop.win32.generated.NativeClipboardSetItem
import org.jetbrains.desktop.win32.generated.NativeClipboardStringArrayResult
import org.jetbrains.desktop.win32.generated.NativeClipboardStringResult
import org.jetbrains.desktop.win32.generated.NativeClipboardUInt32ArrayResult
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit

private const val CLIPBOARD_WRITE_DEPRECATION =
    "Use Clipboard.writeAsync(application, owner) { ... } to publish all formats in one clipboard transaction."
private const val OLE_CLIPBOARD_WRITE_DEPRECATION =
    "Use OleClipboard.writeDataObjectAsync(application, dataObject), or OleClipboard.writeAsync(application) { ... } for new clipboard content."
private const val SYNC_CLIPBOARD_DEPRECATION = "Use the async clipboard APIs; synchronous clipboard APIs are compatibility APIs " +
    "that fail fast under clipboard contention."
private const val CLIPBOARD_ASYNC_THREAD_ERROR = "Clipboard async APIs must be called from the application dispatcher thread."

internal sealed interface ClipboardWriteItem {
    data class Bytes(
        val format: DataFormat,
        val data: ByteArray,
    ) : ClipboardWriteItem
    data class Text(val text: String) : ClipboardWriteItem
    data class HtmlFragment(val fragment: String) : ClipboardWriteItem
    data class FileList(val fileNames: List<String>) : ClipboardWriteItem
}

public class ClipboardWriter internal constructor() {
    private val items = mutableListOf<ClipboardWriteItem>()
    private val formats = mutableSetOf<DataFormat>()

    public fun setItemOfType(format: DataFormat, data: ByteArray) {
        add(format, ClipboardWriteItem.Bytes(format, data.copyOf()))
    }

    public fun setText(text: String) {
        add(DataFormat.Text, ClipboardWriteItem.Text(text))
    }

    public fun setHtmlFragment(fragment: String) {
        add(DataFormat.Html, ClipboardWriteItem.HtmlFragment(fragment))
    }

    public fun setListOfFiles(fileNames: List<String>) {
        add(DataFormat.FileList, ClipboardWriteItem.FileList(fileNames.toList()))
    }

    internal fun build(): List<ClipboardWriteItem> = items.toList()

    private fun add(format: DataFormat, item: ClipboardWriteItem) {
        require(formats.add(format)) { "Clipboard format $format has already been set" }
        items.add(item)
    }
}

/**
 * Win32 clipboard access.
 *
 * Dispatcher-thread-only. Async methods retry busy failures without blocking between attempts.
 */
@Suppress("DEPRECATION")
public object Clipboard {
    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun clear(owner: Window) {
        owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_empty_result(arena, windowPtr)
                }
                checkClipboardOperation(result)
            }
        }
    }

    public fun changeCount(): UInt {
        return ffiDownCall {
            desktop_win32_h.clipboard_get_sequence_number().toUInt()
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun itemCount(owner: Window): Int {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_count_formats_result(arena, windowPtr)
                }
                checkClipboardOperation(NativeClipboardIntResult.result(result))
                NativeClipboardIntResult.value(result)
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun isFormatAvailable(owner: Window, format: DataFormat): Boolean {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_is_format_available_result(arena, windowPtr, format.id)
                }
                checkClipboardOperation(NativeClipboardBoolResult.result(result))
                NativeClipboardBoolResult.value(result)
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun listItemFormats(owner: Window): List<DataFormat> {
        val formatIds = owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_enum_formats_result(arena, windowPtr)
                }
                checkClipboardOperation(NativeClipboardUInt32ArrayResult.result(result))
                val formatsPtr = NativeClipboardUInt32ArrayResult.value(result)
                try {
                    intArrayFromNative(formatsPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_u32_array_drop(formatsPtr)
                    }
                }
            }
        }
        return formatIds.map(DataFormat::fromNative)
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun readItemOfType(owner: Window, format: DataFormat): ByteArray {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_data_result(arena, windowPtr, format.id)
                }
                checkClipboardOperation(NativeClipboardByteArrayResult.result(result))
                val dataPtr = NativeClipboardByteArrayResult.value(result)
                try {
                    byteArrayFromNative(dataPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun tryReadItemOfType(owner: Window, format: DataFormat): ByteArray? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_data_result(arena, windowPtr, format.id)
                }
                val operation = clipboardOperationFromNative(NativeClipboardByteArrayResult.result(result))
                if (!operation.requireOkOrUnavailable()) {
                    return@use null
                }
                val dataPtr = NativeClipboardByteArrayResult.value(result)
                try {
                    byteArrayFromNative(dataPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun readHtmlFragment(owner: Window): String {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_html_fragment_result(arena, windowPtr)
                }
                checkClipboardOperation(NativeClipboardStringResult.result(result))
                stringFromNative(NativeClipboardStringResult.value(result))
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun tryReadHtmlFragment(owner: Window): String? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_html_fragment_result(arena, windowPtr)
                }
                val operation = clipboardOperationFromNative(NativeClipboardStringResult.result(result))
                if (!operation.requireOkOrUnavailable()) {
                    return@use null
                }
                stringFromNative(NativeClipboardStringResult.value(result))
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun readListOfFiles(owner: Window): List<String> {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_file_list_result(arena, windowPtr)
                }
                checkClipboardOperation(NativeClipboardStringArrayResult.result(result))
                val arrayPtr = NativeClipboardStringArrayResult.value(result)
                listOfStringsFromNative(arrayPtr)
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun tryReadListOfFiles(owner: Window): List<String>? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_file_list_result(arena, windowPtr)
                }
                val operation = clipboardOperationFromNative(NativeClipboardStringArrayResult.result(result))
                if (!operation.requireOkOrUnavailable()) {
                    return@use null
                }
                listOfStringsFromNative(NativeClipboardStringArrayResult.value(result))
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun readTextItem(owner: Window): String {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_text_result(arena, windowPtr)
                }
                checkClipboardOperation(NativeClipboardStringResult.result(result))
                stringFromNative(NativeClipboardStringResult.value(result))
            }
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun tryReadTextItem(owner: Window): String? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_text_result(arena, windowPtr)
                }
                val operation = clipboardOperationFromNative(NativeClipboardStringResult.result(result))
                if (!operation.requireOkOrUnavailable()) {
                    return@use null
                }
                stringFromNative(NativeClipboardStringResult.value(result))
            }
        }
    }

    private fun itemCountIfUnchanged(owner: Window, expectedChangeCount: UInt): Int {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_count_formats_if_unchanged_result(arena, windowPtr, expectedChangeCount.toInt())
                }
                checkClipboardReadOperation(NativeClipboardIntResult.result(result), expectedChangeCount)
                NativeClipboardIntResult.value(result)
            }
        }
    }

    private fun isFormatAvailableIfUnchanged(owner: Window, format: DataFormat, expectedChangeCount: UInt): Boolean {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_is_format_available_if_unchanged_result(
                        arena,
                        windowPtr,
                        format.id,
                        expectedChangeCount.toInt(),
                    )
                }
                checkClipboardReadOperation(NativeClipboardBoolResult.result(result), expectedChangeCount)
                NativeClipboardBoolResult.value(result)
            }
        }
    }

    private fun listItemFormatsIfUnchanged(owner: Window, expectedChangeCount: UInt): List<DataFormat> {
        val formatIds = owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_enum_formats_if_unchanged_result(arena, windowPtr, expectedChangeCount.toInt())
                }
                checkClipboardReadOperation(NativeClipboardUInt32ArrayResult.result(result), expectedChangeCount)
                val formatsPtr = NativeClipboardUInt32ArrayResult.value(result)
                try {
                    intArrayFromNative(formatsPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_u32_array_drop(formatsPtr)
                    }
                }
            }
        }
        return formatIds.map(DataFormat::fromNative)
    }

    private fun readItemOfTypeIfUnchanged(owner: Window, format: DataFormat, expectedChangeCount: UInt): ByteArray {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_data_if_unchanged_result(arena, windowPtr, format.id, expectedChangeCount.toInt())
                }
                checkClipboardReadOperation(NativeClipboardByteArrayResult.result(result), expectedChangeCount)
                val dataPtr = NativeClipboardByteArrayResult.value(result)
                try {
                    byteArrayFromNative(dataPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    private fun tryReadItemOfTypeIfUnchanged(owner: Window, format: DataFormat, expectedChangeCount: UInt): ByteArray? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_data_if_unchanged_result(arena, windowPtr, format.id, expectedChangeCount.toInt())
                }
                val operation = clipboardOperationFromNative(NativeClipboardByteArrayResult.result(result))
                if (!operation.requireOkOrUnavailable(expectedChangeCount)) {
                    return@use null
                }
                val dataPtr = NativeClipboardByteArrayResult.value(result)
                try {
                    byteArrayFromNative(dataPtr)
                } finally {
                    ffiDownCall {
                        desktop_win32_h.native_byte_array_drop(dataPtr)
                    }
                }
            }
        }
    }

    private fun readHtmlFragmentIfUnchanged(owner: Window, expectedChangeCount: UInt): String {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_html_fragment_if_unchanged_result(arena, windowPtr, expectedChangeCount.toInt())
                }
                checkClipboardReadOperation(NativeClipboardStringResult.result(result), expectedChangeCount)
                stringFromNative(NativeClipboardStringResult.value(result))
            }
        }
    }

    private fun tryReadHtmlFragmentIfUnchanged(owner: Window, expectedChangeCount: UInt): String? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_html_fragment_if_unchanged_result(arena, windowPtr, expectedChangeCount.toInt())
                }
                val operation = clipboardOperationFromNative(NativeClipboardStringResult.result(result))
                if (!operation.requireOkOrUnavailable(expectedChangeCount)) {
                    return@use null
                }
                stringFromNative(NativeClipboardStringResult.value(result))
            }
        }
    }

    private fun readListOfFilesIfUnchanged(owner: Window, expectedChangeCount: UInt): List<String> {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_file_list_if_unchanged_result(arena, windowPtr, expectedChangeCount.toInt())
                }
                checkClipboardReadOperation(NativeClipboardStringArrayResult.result(result), expectedChangeCount)
                listOfStringsFromNative(NativeClipboardStringArrayResult.value(result))
            }
        }
    }

    private fun tryReadListOfFilesIfUnchanged(owner: Window, expectedChangeCount: UInt): List<String>? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_file_list_if_unchanged_result(arena, windowPtr, expectedChangeCount.toInt())
                }
                val operation = clipboardOperationFromNative(NativeClipboardStringArrayResult.result(result))
                if (!operation.requireOkOrUnavailable(expectedChangeCount)) {
                    return@use null
                }
                listOfStringsFromNative(NativeClipboardStringArrayResult.value(result))
            }
        }
    }

    private fun readTextItemIfUnchanged(owner: Window, expectedChangeCount: UInt): String {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_text_if_unchanged_result(arena, windowPtr, expectedChangeCount.toInt())
                }
                checkClipboardReadOperation(NativeClipboardStringResult.result(result), expectedChangeCount)
                stringFromNative(NativeClipboardStringResult.value(result))
            }
        }
    }

    private fun tryReadTextItemIfUnchanged(owner: Window, expectedChangeCount: UInt): String? {
        return owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_get_text_if_unchanged_result(arena, windowPtr, expectedChangeCount.toInt())
                }
                val operation = clipboardOperationFromNative(NativeClipboardStringResult.result(result))
                if (!operation.requireOkOrUnavailable(expectedChangeCount)) {
                    return@use null
                }
                stringFromNative(NativeClipboardStringResult.value(result))
            }
        }
    }

    @Deprecated(CLIPBOARD_WRITE_DEPRECATION)
    public fun writeItemOfType(owner: Window, format: DataFormat, data: ByteArray) {
        writeItems(owner, listOf(ClipboardWriteItem.Bytes(format, data)))
    }

    @Deprecated(CLIPBOARD_WRITE_DEPRECATION)
    public fun writeHtmlFragment(owner: Window, fragment: String) {
        writeItems(owner, listOf(ClipboardWriteItem.HtmlFragment(fragment)))
    }

    @Deprecated(CLIPBOARD_WRITE_DEPRECATION)
    public fun writeListOfFiles(owner: Window, fileNames: List<String>) {
        writeItems(owner, listOf(ClipboardWriteItem.FileList(fileNames)))
    }

    @Deprecated(CLIPBOARD_WRITE_DEPRECATION)
    public fun writeTextItem(owner: Window, text: String) {
        writeItems(owner, listOf(ClipboardWriteItem.Text(text)))
    }

    private fun writeItems(owner: Window, items: List<ClipboardWriteItem>) {
        require(items.isNotEmpty()) { "items must not be empty; use Clipboard.clear(owner) to clear the clipboard" }
        owner.withPointer { windowPtr ->
            Arena.ofConfined().use { arena ->
                val itemsPtr = clipboardWriteItemsToNative(arena, items)
                val result = ffiDownCall {
                    desktop_win32_h.clipboard_set_items_result(arena, windowPtr, itemsPtr)
                }
                checkClipboardOperation(result)
            }
        }
    }

    public fun clearAsync(application: Application, owner: Window): CompletableFuture<Unit> =
        retryClipboardWriteOperationAsync(application) { clear(owner) }

    public fun itemCountAsync(application: Application, owner: Window): CompletableFuture<Int> =
        retryClipboardReadOperationAsync(application) { expected -> itemCountIfUnchanged(owner, expected) }

    public fun isFormatAvailableAsync(application: Application, owner: Window, format: DataFormat): CompletableFuture<Boolean> =
        retryClipboardReadOperationAsync(application) { expected -> isFormatAvailableIfUnchanged(owner, format, expected) }

    public fun listItemFormatsAsync(application: Application, owner: Window): CompletableFuture<List<DataFormat>> =
        retryClipboardReadOperationAsync(application) { expected -> listItemFormatsIfUnchanged(owner, expected) }

    public fun readItemOfTypeAsync(application: Application, owner: Window, format: DataFormat): CompletableFuture<ByteArray> =
        retryClipboardReadOperationAsync(application) { expected -> readItemOfTypeIfUnchanged(owner, format, expected) }

    public fun tryReadItemOfTypeAsync(application: Application, owner: Window, format: DataFormat): CompletableFuture<ByteArray?> =
        retryClipboardReadOperationAsync(application) { expected -> tryReadItemOfTypeIfUnchanged(owner, format, expected) }

    public fun readHtmlFragmentAsync(application: Application, owner: Window): CompletableFuture<String> =
        retryClipboardReadOperationAsync(application) { expected -> readHtmlFragmentIfUnchanged(owner, expected) }

    public fun tryReadHtmlFragmentAsync(application: Application, owner: Window): CompletableFuture<String?> =
        retryClipboardReadOperationAsync(application) { expected -> tryReadHtmlFragmentIfUnchanged(owner, expected) }

    public fun readListOfFilesAsync(application: Application, owner: Window): CompletableFuture<List<String>> =
        retryClipboardReadOperationAsync(application) { expected -> readListOfFilesIfUnchanged(owner, expected) }

    public fun tryReadListOfFilesAsync(application: Application, owner: Window): CompletableFuture<List<String>?> =
        retryClipboardReadOperationAsync(application) { expected -> tryReadListOfFilesIfUnchanged(owner, expected) }

    public fun readTextItemAsync(application: Application, owner: Window): CompletableFuture<String> =
        retryClipboardReadOperationAsync(application) { expected -> readTextItemIfUnchanged(owner, expected) }

    public fun tryReadTextItemAsync(application: Application, owner: Window): CompletableFuture<String?> =
        retryClipboardReadOperationAsync(application) { expected -> tryReadTextItemIfUnchanged(owner, expected) }

    public fun writeAsync(application: Application, owner: Window, block: ClipboardWriter.() -> Unit): CompletableFuture<Unit> {
        clipboardAsyncWrongThreadFuture<Unit>(application)?.let { return it }
        val items = try {
            clipboardWriteItems(block)
        } catch (t: Throwable) {
            return failedFuture(t)
        }
        return retryClipboardWriteOperationAsync(application) { writeItems(owner, items) }
    }
}

private val clipboardRetryDelaysMs = longArrayOf(10, 25, 50, 100, 200, 400, 800)

private fun <T> retryClipboardReadOperationAsync(application: Application, operation: (UInt) -> T): CompletableFuture<T> =
    retryClipboardOperationAsync(application, ClipboardQueuedOperationKind.Read) { readSequence ->
        operation(checkNotNull(readSequence))
    }

private fun <T> retryClipboardWriteOperationAsync(application: Application, operation: () -> T): CompletableFuture<T> =
    retryClipboardOperationAsync(application, ClipboardQueuedOperationKind.Write) { operation() }

private fun <T> retryClipboardOperationAsync(
    application: Application,
    kind: ClipboardQueuedOperationKind,
    operation: (UInt?) -> T,
): CompletableFuture<T> {
    clipboardAsyncWrongThreadFuture<T>(application)?.let { return it }
    return application.clipboardOperations.enqueue(kind, operation)
}

internal typealias ClipboardRetryDispatcher = (() -> Unit) -> Boolean

internal class ClipboardOperationQueue(
    private val dispatchRetry: ClipboardRetryDispatcher,
    private val readChangeCount: () -> UInt = Clipboard::changeCount,
) {
    constructor(application: Application) : this(application::tryInvokeOnDispatcher)

    // Normal execution is dispatcher-thread-owned. While a retry is waiting to be
    // dispatched, the delayed executor may terminally fail and drain pending operations.
    private val retryDispatchLock = Any()
    private val queue = ArrayDeque<QueuedClipboardOperation<*>>()
    private var active: QueuedClipboardOperation<*>? = null
    @Volatile
    private var retryDispatchPending = false

    fun <T> enqueue(kind: ClipboardQueuedOperationKind, operation: (UInt?) -> T): CompletableFuture<T> {
        val entry = QueuedClipboardOperation(kind, operation)
        if (retryDispatchPending) {
            enqueueWhileRetryDispatchPending(entry)
        } else {
            queue.addLast(entry)
            runNextIfIdle()
        }
        return entry.future
    }

    private fun enqueueWhileRetryDispatchPending(entry: QueuedClipboardOperation<*>) {
        val shouldRun = synchronized(retryDispatchLock) {
            queue.addLast(entry)
            !retryDispatchPending
        }
        if (shouldRun) {
            runNextIfIdle()
        }
    }

    private fun runNextIfIdle() {
        if (active != null) {
            return
        }

        while (queue.isNotEmpty()) {
            val next = queue.removeFirst()
            if (next.future.isDone) {
                continue
            }
            active = next
            start(next)
            return
        }
    }

    private fun start(entry: QueuedClipboardOperation<*>) {
        try {
            entry.readSequence = if (entry.kind == ClipboardQueuedOperationKind.Read) readChangeCount() else null
            attempt(entry, 0)
        } catch (t: Throwable) {
            entry.future.completeExceptionally(t)
            finish(entry)
        }
    }

    private fun <T> attempt(entry: QueuedClipboardOperation<T>, attemptIndex: Int) {
        if (entry.future.isDone) {
            finish(entry)
            return
        }

        try {
            val readSequence = entry.readSequence
            readSequence?.let { expected ->
                if (attemptIndex > 0) {
                    val actual = readChangeCount()
                    if (actual != expected) {
                        entry.future.completeExceptionally(ClipboardChangedException(expected, actual))
                        finish(entry)
                        return
                    }
                }
            }

            val value = entry.operation(readSequence)
            // If the future was already completed elsewhere (e.g. timed out or cancelled),
            // complete() is a no-op and `value` is dropped on the floor. Close it when it
            // owns a native resource so the reference cannot leak (only DataObject reads do).
            if (!entry.future.complete(value) && value is AutoCloseable) {
                value.close()
            }
            finish(entry)
        } catch (e: ClipboardException) {
            val nextAttempt = attemptIndex + 1
            if (e.status == ClipboardStatus.Busy && attemptIndex < clipboardRetryDelaysMs.size) {
                scheduleRetry(entry, clipboardRetryDelaysMs[attemptIndex]) {
                    attempt(entry, nextAttempt)
                }
            } else {
                entry.future.completeExceptionally(e)
                finish(entry)
            }
        } catch (t: Throwable) {
            entry.future.completeExceptionally(t)
            finish(entry)
        }
    }

    private fun scheduleRetry(entry: QueuedClipboardOperation<*>, delayMs: Long, retry: () -> Unit) {
        retryDispatchPending = true
        CompletableFuture.delayedExecutor(delayMs, TimeUnit.MILLISECONDS).execute {
            try {
                val dispatched = dispatchRetry {
                    retryDispatchPending = false
                    if (entry.future.isDone) {
                        finish(entry)
                    } else {
                        retry()
                    }
                }
                if (!dispatched) {
                    failQueueTerminally(
                        entry,
                        IllegalStateException("Failed to dispatch clipboard retry to the application dispatcher."),
                    )
                }
            } catch (t: Throwable) {
                failQueueTerminally(
                    entry,
                    IllegalStateException("Failed to dispatch clipboard retry to the application dispatcher.", t),
                )
            }
        }
    }

    private fun failQueueTerminally(entry: QueuedClipboardOperation<*>, failure: Throwable) {
        val failedEntries = synchronized(retryDispatchLock) {
            if (!retryDispatchPending || active !== entry) {
                return@synchronized listOf(entry)
            }
            val failedEntries = mutableListOf(entry)
            while (queue.isNotEmpty()) {
                val pending = queue.removeFirst()
                failedEntries.add(pending)
            }
            active = null
            retryDispatchPending = false
            failedEntries
        }
        for (failedEntry in failedEntries) {
            failedEntry.future.completeExceptionally(failure)
        }
    }

    private fun finish(entry: QueuedClipboardOperation<*>) {
        if (active !== entry) {
            return
        }
        active = null
        runNextIfIdle()
    }
}

internal enum class ClipboardQueuedOperationKind {
    Read,
    Write,
}

private class QueuedClipboardOperation<T>(
    val kind: ClipboardQueuedOperationKind,
    val operation: (UInt?) -> T,
) {
    val future: CompletableFuture<T> = CompletableFuture()
    var readSequence: UInt? = null
}

private fun <T> failedFuture(error: Throwable): CompletableFuture<T> {
    val future = CompletableFuture<T>()
    future.completeExceptionally(error)
    return future
}

private fun <T> clipboardAsyncWrongThreadFuture(application: Application): CompletableFuture<T>? = try {
    if (application.isDispatcherThread()) {
        null
    } else {
        failedFuture(IllegalStateException(CLIPBOARD_ASYNC_THREAD_ERROR))
    }
} catch (t: Throwable) {
    failedFuture(t)
}

private fun clipboardWriteItems(block: ClipboardWriter.() -> Unit): List<ClipboardWriteItem> = ClipboardWriter()
    .apply(block)
    .build()
    .also { items -> require(items.isNotEmpty()) { "At least one clipboard format must be set" } }

private val ClipboardWriteItem.format: DataFormat
    get() = when (this) {
        is ClipboardWriteItem.Bytes -> format
        is ClipboardWriteItem.Text -> DataFormat.Text
        is ClipboardWriteItem.HtmlFragment -> DataFormat.Html
        is ClipboardWriteItem.FileList -> DataFormat.FileList
    }

private fun clipboardWriteItemsToNative(arena: Arena, items: List<ClipboardWriteItem>): MemorySegment {
    val itemsArray = NativeClipboardSetItem.allocateArray(items.size.toLong(), arena)
    val emptyBytes = emptyByteArrayToNative(arena)
    val emptyStrings = emptyStringArrayToNative(arena)

    items.forEachIndexed { index, item ->
        val nativeItem = NativeClipboardSetItem.asSlice(itemsArray, index.toLong())
        NativeClipboardSetItem.data_format(nativeItem, 0)
        NativeClipboardSetItem.bytes(nativeItem, emptyBytes)
        NativeClipboardSetItem.text(nativeItem, MemorySegment.NULL)
        NativeClipboardSetItem.strings(nativeItem, emptyStrings)

        when (item) {
            is ClipboardWriteItem.Bytes -> {
                NativeClipboardSetItem.kind(nativeItem, desktop_win32_h.NativeClipboardSetItemKind_Bytes())
                NativeClipboardSetItem.data_format(nativeItem, item.format.id)
                NativeClipboardSetItem.bytes(nativeItem, item.data.toNative(arena))
            }
            is ClipboardWriteItem.Text -> {
                NativeClipboardSetItem.kind(nativeItem, desktop_win32_h.NativeClipboardSetItemKind_Text())
                NativeClipboardSetItem.text(nativeItem, arena.allocateFrom(item.text))
            }
            is ClipboardWriteItem.HtmlFragment -> {
                NativeClipboardSetItem.kind(nativeItem, desktop_win32_h.NativeClipboardSetItemKind_HtmlFragment())
                NativeClipboardSetItem.text(nativeItem, arena.allocateFrom(item.fragment))
            }
            is ClipboardWriteItem.FileList -> {
                NativeClipboardSetItem.kind(nativeItem, desktop_win32_h.NativeClipboardSetItemKind_FileList())
                NativeClipboardSetItem.strings(nativeItem, listOfStringsToNative(arena, item.fileNames))
            }
        }
    }

    val result = NativeBorrowedArray_ClipboardSetItem.allocate(arena)
    NativeBorrowedArray_ClipboardSetItem.ptr(result, itemsArray)
    NativeBorrowedArray_ClipboardSetItem.len(result, items.size.toLong())
    return result
}

private fun emptyByteArrayToNative(arena: Arena): MemorySegment {
    val result = NativeBorrowedArray_u8.allocate(arena)
    NativeBorrowedArray_u8.ptr(result, MemorySegment.NULL)
    NativeBorrowedArray_u8.len(result, 0)
    return result
}

private fun emptyStringArrayToNative(arena: Arena): MemorySegment {
    val result = NativeBorrowedArray_BorrowedStrPtr.allocate(arena)
    NativeBorrowedArray_BorrowedStrPtr.ptr(result, MemorySegment.NULL)
    NativeBorrowedArray_BorrowedStrPtr.len(result, 0)
    return result
}

/**
 * OLE clipboard access backed by Windows `IDataObject`.
 *
 * Dispatcher-thread-only. Returned [DataObject] values are bound to the application's OLE STA.
 */
@Suppress("DEPRECATION")
public object OleClipboard {
    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun clear() {
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.ole_clipboard_empty_result(arena)
            }
            checkClipboardOperation(result)
        }
    }

    @Deprecated(SYNC_CLIPBOARD_DEPRECATION)
    public fun readClipboard(): DataObject {
        val ptr = Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.ole_clipboard_get_data_result(arena)
            }
            checkClipboardOperation(NativeClipboardDataObjectResult.result(result))
            NativeClipboardDataObjectResult.value(result)
        }
        return DataObject(ptr)
    }

    private fun readClipboardIfUnchanged(expectedChangeCount: UInt): DataObject {
        val ptr = Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.ole_clipboard_get_data_if_unchanged_result(arena, expectedChangeCount.toInt())
            }
            checkClipboardReadOperation(NativeClipboardDataObjectResult.result(result), expectedChangeCount)
            NativeClipboardDataObjectResult.value(result)
        }
        return DataObject(ptr)
    }

    @Deprecated(OLE_CLIPBOARD_WRITE_DEPRECATION)
    public fun writeToClipboard(dataObject: DataObject) {
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.ole_clipboard_set_data_result(arena, dataObject.toNative())
            }
            checkClipboardOperation(result)
        }
    }

    public fun clearAsync(application: Application): CompletableFuture<Unit> = retryClipboardWriteOperationAsync(application) { clear() }

    public fun readClipboardAsync(application: Application): CompletableFuture<DataObject> =
        retryClipboardReadOperationAsync(application) { expected -> readClipboardIfUnchanged(expected) }

    public fun writeAsync(application: Application, block: ClipboardWriter.() -> Unit): CompletableFuture<Unit> {
        clipboardAsyncWrongThreadFuture<Unit>(application)?.let { return it }
        val items = try {
            clipboardWriteItems(block)
        } catch (t: Throwable) {
            return failedFuture(t)
        }
        val dataObject = try {
            DataObject.build {
                addClipboardWriteItems(items)
            }
        } catch (t: Throwable) {
            return failedFuture(t)
        }
        return try {
            writeDataObjectAsync(application, dataObject)
        } finally {
            dataObject.close()
        }
    }

    /**
     * This method retains [dataObject] immediately so callers may close their original
     * wrapper after this method returns. Retaining is a COM operation on the `IDataObject`,
     * so this method must be called from the dispatcher thread.
     */
    public fun writeDataObjectAsync(application: Application, dataObject: DataObject): CompletableFuture<Unit> {
        clipboardAsyncWrongThreadFuture<Unit>(application)?.let { return it }
        val retainedDataObject = try {
            dataObject.retain()
        } catch (t: Throwable) {
            return failedFuture(t)
        }
        val future = retryClipboardWriteOperationAsync(application) { writeToClipboard(retainedDataObject) }
        future.whenComplete { _, _ ->
            closeDataObjectOnDispatcher(application, retainedDataObject)
        }
        return future
    }
}

private fun DataObjectBuilder.addClipboardWriteItems(items: List<ClipboardWriteItem>) {
    for (item in items) {
        val added = when (item) {
            is ClipboardWriteItem.Bytes -> addItemOfType(item.format, item.data)
            is ClipboardWriteItem.Text -> addTextItem(item.text)
            is ClipboardWriteItem.HtmlFragment -> addHtmlFragment(item.fragment)
            is ClipboardWriteItem.FileList -> addListOfFiles(item.fileNames)
        }
        require(added) { "Clipboard format ${item.format} has already been set" }
    }
}

private fun closeDataObjectOnDispatcher(application: Application, dataObject: DataObject) {
    try {
        if (application.isDispatcherThread()) {
            closeDataObject(dataObject)
            return
        }
        if (application.tryInvokeOnDispatcher { closeDataObject(dataObject) }) {
            return
        }
    } catch (t: Throwable) {
        Logger.warn(t) {
            "Failed to schedule retained OLE clipboard data object release on the dispatcher thread; leaking it."
        }
        return
    }
    Logger.warn {
        "Leaking retained OLE clipboard data object because the dispatcher is shut down; " +
            "foreign-thread COM release is unsafe."
    }
}

private fun closeDataObject(dataObject: DataObject) {
    try {
        dataObject.close()
    } catch (t: Throwable) {
        Logger.error(t) { "Failed to release retained OLE clipboard data object" }
    }
}
