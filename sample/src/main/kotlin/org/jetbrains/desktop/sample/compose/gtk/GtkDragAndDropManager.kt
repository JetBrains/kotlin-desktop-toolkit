@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose.gtk

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTransferAction
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.scene.ComposeSceneDragAndDropNode
import androidx.compose.ui.unit.Density
import org.jetbrains.desktop.gtk.DragAndDropAction
import org.jetbrains.desktop.gtk.DragAndDropQueryData
import org.jetbrains.desktop.gtk.DragAndDropQueryResponse
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.SupportedActionsForMime
import org.jetbrains.desktop.sample.compose.ClipboardFormat
import org.jetbrains.desktop.sample.compose.DragAndDropClipboardEntry
import org.jetbrains.desktop.sample.compose.decodeLinuxMimeData
import org.jetbrains.desktop.sample.compose.linuxMimeTypes

/**
 * The clipboard behind an in-flight drag over this window.
 *
 * GTK splits a drop in two: while the drag is only hovering, the target knows the offered MIME types and
 * actions but is given no bytes at all, which is why [data] is null until the drop actually happens. The
 * hover phase is what [acceptsFormat] feeds: the answer the target gives is turned into the toolkit's
 * `DragAndDropQueryResponse`, and that is what decides the drag cursor and whether a drop is allowed at
 * all.
 */
internal class GtkDragAndDropClipboardEntry(
    private val mimeTypes: List<String>,
    private val offeredActions: Set<DragAndDropAction>,
    private val data: ByteArray?,
) : DragAndDropClipboardEntry {
    private val acceptedMimeTypes = linkedMapOf<String, List<DragAndDropTransferAction>>()

    override suspend fun <T : Any> getForFormat(format: ClipboardFormat<T>): List<T> = getForFormatSync(format)

    override fun <T : Any> getForFormatSync(format: ClipboardFormat<T>): List<T> {
        val bytes = data ?: return emptyList()
        // The toolkit delivers exactly the one type it negotiated, so there is nothing to choose here.
        val negotiated = mimeTypes.singleOrNull() ?: return emptyList()
        return if (negotiated in format.linuxMimeTypes()) decodeLinuxMimeData(bytes, format) else emptyList()
    }

    override fun containsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>): Boolean {
        if (actions.none { it.toDragAndDropAction() in offeredActions }) {
            return false
        }
        return format.linuxMimeTypes().any { it in mimeTypes }
    }

    override fun acceptsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>) {
        if (!containsFormat(format, actions)) {
            return
        }
        // Every alias of the format is accepted, so the source may offer whichever one it has.
        for (mimeType in format.linuxMimeTypes()) {
            if (mimeType in mimeTypes) {
                acceptedMimeTypes[mimeType] = actions
            }
        }
    }

    internal fun acceptedMimeTypes(): List<Pair<String, List<DragAndDropTransferAction>>> = acceptedMimeTypes.map { it.key to it.value }
}

/**
 * Drives one window's Compose drag-and-drop node from the toolkit's drag events.
 *
 * The hover phase arrives as a *query* rather than as a stream of enter/move/exit events: the toolkit
 * asks, on every motion, which MIME types and actions the window would take at this position, and this
 * class answers by walking the Compose node — start and move it, then ask whether anything under the
 * pointer would take a drop. `onStarted` is therefore sent from the first query and `onEnded` only from the
 * drop or the leave.
 */
internal class GtkDragAndDropManager(
    private val rootDragAndDropNode: () -> ComposeSceneDragAndDropNode,
    private val density: () -> Density,
) {
    private var previousAction: DragAndDropTransferAction? = null
    private var isStarted = false

    fun onQuery(query: DragAndDropQueryData): DragAndDropQueryResponse {
        val entry = GtkDragAndDropClipboardEntry(query.mimeTypes, query.actions, data = null)
        val event = DragAndDropEvent(
            action = query.actions.preferredComposeAction(),
            nativeEvent = entry,
            positionInRootImpl = query.locationInWindow.toPxOffset(density()),
        )
        val node = rootDragAndDropNode()

        if (!isStarted) {
            if (!node.acceptDragAndDropTransfer(event)) {
                return DragAndDropQueryResponse(emptyList())
            }
            isStarted = true
            previousAction = event.action
            node.onStarted(event)
            // `onMoved`, not `onEntered`: only the former updates the node the drag is currently over,
            // which is what `hasEligibleDropTarget` is asked about below.
            node.onMoved(event)
        } else {
            if (event.action != previousAction) {
                node.onChanged(event)
                previousAction = event.action
            }
            node.onMoved(event)
        }

        if (!node.hasEligibleDropTarget) {
            return DragAndDropQueryResponse(emptyList())
        }
        return DragAndDropQueryResponse(
            entry.acceptedMimeTypes().mapNotNull { (mimeType, actions) ->
                val gtkActions = actions.mapNotNull(DragAndDropTransferAction::toDragAndDropAction).toSet()
                if (gtkActions.isEmpty()) {
                    null
                } else {
                    SupportedActionsForMime(
                        supportedMimeType = mimeType,
                        supportedActions = gtkActions,
                        preferredAction = gtkActions.first(),
                    )
                }
            },
        )
    }

    fun onDrop(event: Event.DropPerformed) {
        val content = event.content
        val dragEvent = DragAndDropEvent(
            action = event.action?.toDragAndDropTransferAction(),
            nativeEvent = GtkDragAndDropClipboardEntry(
                mimeTypes = listOfNotNull(content?.mimeType),
                offeredActions = setOfNotNull(event.action),
                data = content?.data,
            ),
            positionInRootImpl = event.locationInWindow.toPxOffset(density()),
        )
        val node = rootDragAndDropNode()
        node.onDrop(dragEvent)
        node.onEnded(dragEvent)
        previousAction = null
        isStarted = false
    }

    fun onLeave() {
        if (!isStarted) return
        val event = DragAndDropEvent(
            action = null,
            nativeEvent = GtkDragAndDropClipboardEntry(emptyList(), emptySet(), data = null),
            positionInRootImpl = Offset.Zero,
        )
        val node = rootDragAndDropNode()
        node.onExited(event)
        node.onEnded(event)
        previousAction = null
        isStarted = false
    }
}

/** Move wins over copy, matching the order the other backends resolve an ambiguous offer in. */
private fun Set<DragAndDropAction>.preferredComposeAction(): DragAndDropTransferAction? = when {
    DragAndDropAction.Move in this -> DragAndDropTransferAction.Move
    DragAndDropAction.Copy in this -> DragAndDropTransferAction.Copy
    else -> null
}

internal fun DragAndDropTransferAction.toDragAndDropAction(): DragAndDropAction? = when (this) {
    DragAndDropTransferAction.Copy -> DragAndDropAction.Copy
    DragAndDropTransferAction.Move -> DragAndDropAction.Move
    // GTK has no counterpart for Link.
    else -> null
}

internal fun DragAndDropAction.toDragAndDropTransferAction(): DragAndDropTransferAction = when (this) {
    DragAndDropAction.Copy -> DragAndDropTransferAction.Copy
    DragAndDropAction.Move -> DragAndDropTransferAction.Move
}
