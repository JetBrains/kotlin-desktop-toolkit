@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.compose.gtk

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.Clipboard
import kotlinx.coroutines.suspendCancellableCoroutine
import org.jetbrains.desktop.compose.ClipboardEntry
import org.jetbrains.desktop.compose.ClipboardFormat
import org.jetbrains.desktop.compose.ClipboardItemsEntry
import org.jetbrains.desktop.compose.dataForLinuxMimeType
import org.jetbrains.desktop.compose.decodeLinuxMimeData
import org.jetbrains.desktop.compose.linuxMimeTypes
import org.jetbrains.desktop.gtk.DataSource
import org.jetbrains.desktop.gtk.DataTransferContent
import org.jetbrains.desktop.gtk.Event
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import kotlin.coroutines.resume

/**
 * The GTK clipboard, which like Wayland's is a protocol rather than a buffer.
 *
 * Nothing is ever "on the clipboard": a copy only tells the desktop which MIME types this application
 * *could* produce, and the bytes are asked for later — [dataForMimeType] is that answer, driven from
 * [GtkApplication]'s `getDataTransferData` callback. A paste is the mirror image and equally asynchronous:
 * [getClipEntry] hands back an entry whose reads post a request and suspend until an `Event.DataTransfer`
 * arrives with the matching serial.
 *
 * That is why reads have to be `suspend` here and cannot be answered synchronously — a synchronous read
 * would have to block the event loop thread that the answer arrives on. Compose only reads the clipboard
 * from a coroutine, so this costs nothing in practice.
 *
 * The primary selection (middle-click paste) is a second, independent clipboard. The toolkit exposes it,
 * but Compose's `Clipboard` has no concept of it, so it is left out of this layer.
 */
internal class GtkClipboard(private val application: GtkApplication) : Clipboard {
    private val serials = AtomicInteger(0)
    private val pendingReads = ConcurrentHashMap<Int, (DataTransferContent?) -> Unit>()

    private val lock = Any()

    /** Set while this application is the clipboard owner, so a paste into ourselves needs no round trip. */
    private var ownedEntry: ClipboardItemsEntry? = null
    private var availableMimeTypes: List<String> = emptyList()

    private val remoteEntry = RemoteClipboardEntry()

    override suspend fun getClipEntry(): ClipEntry = ClipEntry(synchronized(lock) { ownedEntry } ?: remoteEntry)

    override suspend fun setClipEntry(clipEntry: ClipEntry?) {
        val entry = clipEntry?.nativeClipEntry as? ClipboardItemsEntry ?: return
        val mimeTypes = entry.linuxMimeTypes()
        synchronized(lock) {
            ownedEntry = entry
        }
        application.onEventLoopAsync {
            clipboardPut(mimeTypes)
        }
    }

    override val nativeClipboard: Any get() = application.nativeApplication

    // ----- Event plumbing, all on the event loop thread -----

    /** The desktop announced what the current clipboard owner can produce. */
    fun onDataTransferAvailable(event: Event.DataTransferAvailable) {
        if (event.dataSource != DataSource.Clipboard) return
        synchronized(lock) {
            availableMimeTypes = event.mimeTypes
            // The GTK backend does not send DataTransferCancelled for the clipboard, so losing ownership
            // has to be read off an announcement that offers nothing.
            if (event.mimeTypes.isEmpty()) {
                ownedEntry = null
            }
        }
    }

    /** Answers one pending read; `false` when the serial belongs to somebody else. */
    fun onDataTransfer(event: Event.DataTransfer): Boolean {
        val receiver = pendingReads.remove(event.serial) ?: return false
        receiver(event.content)
        return true
    }

    /** Somebody else took ownership of the clipboard, so our data is no longer what is on it. */
    fun onOwnershipLost() {
        synchronized(lock) {
            ownedEntry = null
        }
    }

    /** The bytes another application asked for, while this one owns the clipboard. */
    fun dataForMimeType(mimeType: String): ByteArray? = synchronized(lock) {
        ownedEntry?.dataForLinuxMimeType(mimeType)
    }

    private inner class RemoteClipboardEntry : ClipboardEntry {
        override suspend fun <T : Any> getForFormat(format: ClipboardFormat<T>): List<T> {
            val available = synchronized(lock) { availableMimeTypes }
            val mimeTypes = format.linuxMimeTypes().filter { it in available }
            if (mimeTypes.isEmpty()) {
                return emptyList()
            }
            return suspendCancellableCoroutine { continuation ->
                application.onEventLoopAsync {
                    val serial = serials.incrementAndGet()
                    pendingReads[serial] = { content ->
                        // Only the types we asked for can be decoded as this format; anything else would
                        // be read as garbage. Empty rather than an exception: this resumes inside a
                        // Compose paste handler.
                        val answered = content?.takeIf { it.mimeType in mimeTypes }
                        continuation.resume(answered?.let { decodeLinuxMimeData(it.data, format) }.orEmpty())
                    }
                    continuation.invokeOnCancellation { pendingReads.remove(serial) }
                    clipboardPaste(serial, mimeTypes)
                }
            }
        }

        /** See the class comment: a GTK paste cannot be answered without suspending. */
        override fun <T : Any> getForFormatSync(format: ClipboardFormat<T>): List<T> = emptyList()
    }
}
