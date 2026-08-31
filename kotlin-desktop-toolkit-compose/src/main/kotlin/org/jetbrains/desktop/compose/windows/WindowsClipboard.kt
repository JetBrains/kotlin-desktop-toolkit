@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.compose.windows

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.Clipboard
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import org.jetbrains.desktop.compose.ClipboardEntry
import org.jetbrains.desktop.compose.ClipboardFormat
import org.jetbrains.desktop.compose.ClipboardItem
import org.jetbrains.desktop.compose.ClipboardItemsEntry
import org.jetbrains.desktop.compose.ComposeApplication
import org.jetbrains.desktop.compose.KdtImmediateDispatcher
import org.jetbrains.desktop.win32.ClipboardResult
import org.jetbrains.desktop.win32.DataObject
import org.jetbrains.desktop.win32.DataObjectBuilder
import org.jetbrains.desktop.win32.DataTransferException
import org.jetbrains.desktop.win32.Logger
import org.jetbrains.desktop.win32.isBusy
import java.lang.ref.Cleaner
import kotlin.time.Duration.Companion.milliseconds
import org.jetbrains.desktop.win32.Clipboard as Win32Clipboard
import org.jetbrains.desktop.win32.DataFormat as Win32DataFormat

/**
 * The OLE clipboard, as Compose's [Clipboard].
 *
 * Two things about the Windows clipboard shape this class:
 *
 *  - it is *borrowed*, not read: `OleGetClipboard` hands back an `IDataObject` that renders its formats
 *    on demand, so a read is a live object with an owner — see [WindowsClipboardEntry];
 *  - any process can hold it open for a moment, which OLE reports as a plain failure. The toolkit
 *    deliberately leaves retrying to the caller, so every access here goes through [withBusyRetry].
 *
 * Every call must run on the event loop thread, because that is the process's OLE apartment.
 */
internal class WindowsClipboard(private val application: ComposeApplication) : Clipboard {
    override suspend fun getClipEntry(): ClipEntry = withContext(KdtImmediateDispatcher) {
        ClipEntry(WindowsClipboardEntry(withBusyRetry { Win32Clipboard.get() }, application))
    }

    override suspend fun setClipEntry(clipEntry: ClipEntry?) {
        val entry = clipEntry?.nativeClipEntry ?: return
        withContext(KdtImmediateDispatcher) {
            when (entry) {
                is ClipboardItemsEntry -> DataObject.build { addClipboardItems(entry.items) }.use { publish(it) }
                // Copying what was read: the data object is already on the clipboard's terms, so it can
                // be handed straight back without materialising every format in between.
                is WindowsClipboardEntry -> publish(entry.dataObject)
                else -> error("Unsupported clipboard entry: $entry")
            }
        }
    }

    /**
     * `set` publishes the data object with delayed rendering — the clipboard keeps a reference and asks
     * for each format when something pastes it — and `flush` then renders every format into the
     * clipboard itself, which is what keeps the data available after this process exits.
     */
    private suspend fun publish(dataObject: DataObject) {
        withBusyRetry { Win32Clipboard.set(dataObject) }
        withBusyRetry { Win32Clipboard.flush() }
    }

    override val nativeClipboard: Any get() = Win32Clipboard
}

/**
 * A clipboard read: the `IDataObject` the clipboard handed out, plus the formats decoded from it on
 * demand. Reads reflect the clipboard as it was when the entry was created, not as it is now.
 *
 * The entry owns the data object's COM reference, and the reference has to be released on the event loop
 * thread. Compose gives a `ClipEntry` no lifecycle of its own, so nothing will call [close] — a
 * [Cleaner] releases the reference once the entry becomes unreachable instead. A caller that does know
 * the scope may [close] it for a prompt release.
 */
internal class WindowsClipboardEntry(
    internal val dataObject: DataObject,
    application: ComposeApplication,
) : ClipboardEntry,
    AutoCloseable {
    // The cleanup action must not capture `this`, or the entry could never become unreachable.
    private val cleanable: Cleaner.Cleanable = cleaner.register(this, DataObjectReleaser(dataObject, application))

    override suspend fun <T : Any> getForFormat(format: ClipboardFormat<T>): List<T> =
        withContext(KdtImmediateDispatcher) { getForFormatSync(format) }

    override fun <T : Any> getForFormatSync(format: ClipboardFormat<T>): List<T> = dataObject.readForFormat(format)

    override fun close() {
        cleanable.clean()
    }

    private class DataObjectReleaser(
        private val dataObject: DataObject,
        private val application: ComposeApplication,
    ) : Runnable {
        override fun run() {
            // Runs on the cleaner's own thread in the usual case, so it has to hop.
            if (application.isUiThread()) {
                dataObject.close()
            } else {
                application.invokeOnUiThread { dataObject.close() }
            }
        }
    }

    private companion object {
        val cleaner: Cleaner = Cleaner.create()
    }
}

/**
 * PNG has no clipboard format of its own on Windows — `CF_BITMAP` and `CF_DIB` are decoded bitmaps — so
 * it travels under its registered name, which is what browsers and image editors publish it as.
 */
private val pngFormat by lazy { Win32DataFormat.register("PNG") }

internal fun ClipboardFormat<*>.toWin32DataFormat(): Win32DataFormat = when (this) {
    ClipboardFormat.Utf8PlainText -> Win32DataFormat.Text
    ClipboardFormat.Html -> Win32DataFormat.Html
    ClipboardFormat.File -> Win32DataFormat.FileList
    ClipboardFormat.Png -> pngFormat
}

/**
 * Reads [format] out of a data object, which may equally be a clipboard read or an in-flight drop.
 *
 * A format that is simply not on offer comes back empty rather than throwing — that is the normal case
 * when asking for one of several — while a transfer that failed for any other reason is logged and
 * treated the same way, because a paste is not worth taking the application down for.
 */
internal fun <T : Any> DataObject.readForFormat(format: ClipboardFormat<T>): List<T> {
    @Suppress("UNCHECKED_CAST")
    return try {
        when (format) {
            ClipboardFormat.Utf8PlainText -> listOfNotNull(tryReadTextItem())
            ClipboardFormat.Html -> listOfNotNull(tryReadHtmlFragment())
            ClipboardFormat.File -> tryReadListOfFiles().orEmpty()
            ClipboardFormat.Png -> listOfNotNull(tryReadItemOfType(pngFormat))
        } as List<T>
    } catch (exception: DataTransferException) {
        Logger.warn(exception) { "Could not read $format from the data object" }
        emptyList()
    }
}

internal fun DataObjectBuilder.addClipboardItems(items: List<ClipboardItem>) {
    // Windows offers one item per format, not the per-item bundles macOS and Wayland carry, so a
    // multi-item entry collapses: the last value written for a format is the one that is offered.
    items.flatMap { it.elements }.forEach { element ->
        when (element.format) {
            ClipboardFormat.Utf8PlainText -> addTextItem(element.value as String)
            ClipboardFormat.Html -> addHtmlFragment(element.value as String)
            ClipboardFormat.File -> addListOfFiles(listOf(element.value as String))
            ClipboardFormat.Png -> addItemOfType(pngFormat, element.value as ByteArray)
        }
    }
}

/**
 * Retries an operation that failed only because another process had the clipboard open, backing off a
 * little further each time. Any other failure is final.
 */
private suspend fun <T> withBusyRetry(operation: () -> ClipboardResult<T>): T {
    var lastFailure: ClipboardResult.Failure? = null
    for (retryDelay in retryDelays) {
        when (val result = operation()) {
            is ClipboardResult.Success -> return result.value
            is ClipboardResult.Failure -> {
                check(result.isBusy) { "Clipboard operation failed: ${result.status} (0x${result.nativeCode.toHexString()})" }
                lastFailure = result
                delay(retryDelay)
            }
        }
    }
    error("Clipboard stayed busy, last failure: $lastFailure")
}

private val retryDelays = listOf(
    10.milliseconds,
    25.milliseconds,
    50.milliseconds,
    100.milliseconds,
    200.milliseconds,
)
