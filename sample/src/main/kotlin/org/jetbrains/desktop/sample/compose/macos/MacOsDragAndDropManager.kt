@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose.macos

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTransferAction
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.scene.ComposeSceneDragAndDropNode
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.DpOffset
import org.jetbrains.desktop.macos.DragInfo
import org.jetbrains.desktop.macos.DragOperation
import org.jetbrains.desktop.macos.Pasteboard
import org.jetbrains.desktop.macos.PasteboardType
import org.jetbrains.desktop.sample.compose.ClipboardFormat
import org.jetbrains.desktop.sample.compose.DragAndDropClipboardEntry

private fun PasteboardType.containsUniformTypeIdentifier(target: String): Boolean {
    val itemCount = Pasteboard.itemCount(this).toInt()
    return (0 until itemCount).any { target in Pasteboard.readItemTypes(it, this) }
}

internal class MacOsDragAndDropClipboardEntry(private val dragInfo: DragInfo) : DragAndDropClipboardEntry {
    private val clipboardEntry = MacOsClipboardEntry(dragInfo.pasteboard)
    private val acceptedTargets = linkedMapOf<String, List<DragOperation>>()

    override suspend fun <T : Any> getForFormat(format: ClipboardFormat<T>): List<T> {
        return clipboardEntry.getForFormat(format)
    }

    override fun <T : Any> getForFormatSync(format: ClipboardFormat<T>): List<T> {
        return clipboardEntry.getForFormatSync(format)
    }

    override fun containsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>): Boolean {
        val pasteboard = dragInfo.pasteboard
        if (!actions.any { dragInfo.allowedOperations.contains(it.toDragOperation()) }) {
            return false
        }
        val target = format.toUniformTypeIdentifier()
        return pasteboard.containsUniformTypeIdentifier(target)
    }

    override fun acceptsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>) {
        if (containsFormat(format, actions)) {
            val target = format.toUniformTypeIdentifier()
            acceptedTargets[target] = actions.map(DragAndDropTransferAction::toDragOperation)
        }
    }

    internal fun acceptedTargets(): Collection<Pair<String, List<DragOperation>>> = acceptedTargets.map { it.key to it.value }
}

internal class MacOsDragAndDropManager(
    private val rootDragAndDropNode: () -> ComposeSceneDragAndDropNode,
    private val density: () -> Density,
) {
    private var previousAction: DragAndDropTransferAction? = null

    fun onDragEntered(info: DragInfo): DragOperation {
        val dndEvent = info.toDragAndDropEvent()
        val node = rootDragAndDropNode()

        val acceptedTransfer = node.acceptDragAndDropTransfer(dndEvent)
        if (acceptedTransfer) {
            previousAction = dndEvent.action
            node.onStarted(dndEvent)
            node.onEntered(dndEvent)
        }

        val action = dndEvent.action
        return if (acceptedTransfer && action != null) {
            action.toDragOperation()
        } else {
            DragOperation.NONE
        }
    }

    fun onDragUpdated(info: DragInfo): DragOperation {
        val node = rootDragAndDropNode()
        val nativeEvent = MacOsDragAndDropClipboardEntry(info)
        val dndEvent = info.toDragAndDropEvent(nativeEvent)

        if (dndEvent.action != previousAction) {
            node.onChanged(dndEvent)
            previousAction = dndEvent.action
        }
        node.onMoved(dndEvent)

        val acceptedTargets = nativeEvent.acceptedTargets()
        return if (node.hasEligibleDropTarget) {
            val acceptedDragOperations = acceptedTargets.firstNotNullOfOrNull { (target, actions) ->
                if (info.pasteboard.containsUniformTypeIdentifier(target)) {
                    actions.filter { it in info.allowedOperations }.ifEmpty { null }
                } else {
                    null
                }
            }.orEmpty()

            when {
                DragOperation.MOVE in acceptedDragOperations -> DragOperation.MOVE
                DragOperation.COPY in acceptedDragOperations -> DragOperation.COPY
                DragOperation.LINK in acceptedDragOperations -> DragOperation.LINK
                else -> DragOperation.NONE
            }
        } else {
            DragOperation.NONE
        }
    }

    fun onDragExited() {
        val dndEvent = DragAndDropEvent(
            action = null,
            nativeEvent = null,
            positionInRootImpl = Offset.Zero,
        )
        val node = rootDragAndDropNode()

        node.onExited(dndEvent)
        node.onEnded(dndEvent)
        previousAction = null
    }

    fun onDragPerformed(info: DragInfo): Boolean {
        val dndEvent = info.toDragAndDropEvent()
        val node = rootDragAndDropNode()

        val wasDropConsumed = node.onDrop(dndEvent)
        node.onEnded(dndEvent)
        previousAction = null
        return wasDropConsumed
    }

    private fun DragInfo.toDragAndDropEvent(
        nativeEvent: MacOsDragAndDropClipboardEntry = MacOsDragAndDropClipboardEntry(this),
    ): DragAndDropEvent {
        val selectedAction = when {
            DragOperation.MOVE in allowedOperations -> DragAndDropTransferAction.Move
            DragOperation.COPY in allowedOperations -> DragAndDropTransferAction.Copy
            DragOperation.LINK in allowedOperations -> DragAndDropTransferAction.Link
            else -> null
        }

        return DragAndDropEvent(
            action = selectedAction,
            nativeEvent = nativeEvent,
            positionInRootImpl = locationInWindow.toDpOffset().toOffset(),
        )
    }

    private fun DpOffset.toOffset() = toPxOffset(density())
}

internal fun DragAndDropTransferAction.toDragOperation(): DragOperation = when (this) {
    DragAndDropTransferAction.Copy -> DragOperation.COPY
    DragAndDropTransferAction.Link -> DragOperation.LINK
    DragAndDropTransferAction.Move -> DragOperation.MOVE
    else -> DragOperation.NONE
}

internal fun DragOperation.toDragAndDropTransferAction(): DragAndDropTransferAction? = when (this) {
    DragOperation.COPY -> DragAndDropTransferAction.Copy
    DragOperation.LINK -> DragAndDropTransferAction.Link
    DragOperation.MOVE -> DragAndDropTransferAction.Move
    else -> null
}
