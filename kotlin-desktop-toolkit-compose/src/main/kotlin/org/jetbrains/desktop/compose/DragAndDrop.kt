@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.compose

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTransferAction

/**
 * The clipboard behind an in-flight drag, carried as a [DragAndDropEvent]'s `nativeEvent`.
 *
 * A drag is not a plain clipboard read: the drop target can also tell the platform which formats and
 * actions it is willing to take, which is what drives the drag cursor. Compose has no API for that
 * (see CMP-7806), so [acceptsFormat] is offered on the event instead.
 */
interface DragAndDropClipboardEntry : ClipboardEntry {
    /** Whether the drag carries [format] and would allow any of [actions]. */
    fun containsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>): Boolean

    /**
     * Declares that the target would accept [format] with any of [actions]. Call it from `onMoved` to
     * get the matching drag cursor; the last declaration for a given format wins.
     */
    fun acceptsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>)
}

val DragAndDropEvent.clipboardEntry: DragAndDropClipboardEntry
    get() = nativeEvent as DragAndDropClipboardEntry

fun DragAndDropEvent.containsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>): Boolean =
    clipboardEntry.containsFormat(format, actions)

fun DragAndDropEvent.acceptsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>) {
    clipboardEntry.acceptsFormat(format, actions)
}
