package org.jetbrains.desktop.win32

import org.jetbrains.desktop.win32.generated.NativeClipboardDataObjectResult
import org.jetbrains.desktop.win32.generated.desktop_win32_h
import java.lang.foreign.Arena
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit

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
 * Windows clipboard access backed by OLE `IDataObject`.
 *
 * Dispatcher-thread-only. Methods enqueue operations per [Application] and retry busy failures
 * without blocking between attempts.
 */
public object Clipboard {
    public fun changeCount(): UInt {
        return ffiDownCall {
            desktop_win32_h.clipboard_get_sequence_number().toUInt()
        }
    }

    private fun clearNow() {
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.clipboard_clear_result(arena)
            }
            checkClipboardOperation(result)
        }
    }

    private fun readIfUnchanged(expectedChangeCount: UInt): DataObject {
        val ptr = Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.clipboard_read_if_unchanged_result(arena, expectedChangeCount.toInt())
            }
            checkClipboardReadOperation(NativeClipboardDataObjectResult.result(result), expectedChangeCount)
            NativeClipboardDataObjectResult.value(result)
        }
        return DataObject(ptr)
    }

    private fun writeNow(dataObject: DataObject) {
        Arena.ofConfined().use { arena ->
            val result = ffiDownCall {
                desktop_win32_h.clipboard_write_data_object_result(arena, dataObject.toNative())
            }
            checkClipboardOperation(result)
        }
    }

    public fun clearAsync(application: Application): CompletableFuture<Unit> = retryClipboardWriteOperationAsync(application) { clearNow() }

    public fun readAsync(application: Application): CompletableFuture<DataObject> =
        retryClipboardReadOperationAsync(application) { expected -> readIfUnchanged(expected) }

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
     * Retains [dataObject] immediately so callers may close their original wrapper after this
     * method returns. Retaining is a COM operation on the `IDataObject`, so this method must be
     * called from the dispatcher thread.
     */
    public fun writeDataObjectAsync(application: Application, dataObject: DataObject): CompletableFuture<Unit> {
        clipboardAsyncWrongThreadFuture<Unit>(application)?.let { return it }
        val retainedDataObject = try {
            dataObject.retain()
        } catch (t: Throwable) {
            return failedFuture(t)
        }
        val future = retryClipboardWriteOperationAsync(application) { writeNow(retainedDataObject) }
        future.whenComplete { _, _ ->
            closeDataObjectOnDispatcher(application, retainedDataObject)
        }
        return future
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
            // owns a native resource so the reference cannot leak.
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
            "Failed to schedule retained clipboard data object release on the dispatcher thread; leaking it."
        }
        return
    }
    Logger.warn {
        "Leaking retained clipboard data object because the dispatcher is shut down; " +
            "foreign-thread COM release is unsafe."
    }
}

private fun closeDataObject(dataObject: DataObject) {
    try {
        dataObject.close()
    } catch (t: Throwable) {
        Logger.error(t) { "Failed to release retained clipboard data object" }
    }
}
