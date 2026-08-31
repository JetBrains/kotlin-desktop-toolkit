@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.compose.windows

import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTransferAction
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.scene.ComposeSceneDragAndDropNode
import org.jetbrains.desktop.compose.ClipboardFormat
import org.jetbrains.desktop.compose.DragAndDropClipboardEntry
import org.jetbrains.desktop.win32.DataObject
import org.jetbrains.desktop.win32.DataTransferException
import org.jetbrains.desktop.win32.DragDropEffect
import org.jetbrains.desktop.win32.DragDropModifier
import org.jetbrains.desktop.win32.DragDropModifiers
import org.jetbrains.desktop.win32.Logger
import org.jetbrains.desktop.win32.PhysicalPoint
import org.jetbrains.desktop.win32.Screen
import org.jetbrains.desktop.win32.Window
import org.jetbrains.desktop.win32.DataFormat as Win32DataFormat

/**
 * The drop-target half of Windows drag and drop: OLE `IDropTarget` callbacks, translated into the
 * scene's [ComposeSceneDragAndDropNode], and the [DragDropEffect] to answer each of them with.
 *
 * The drag *source* half is [org.jetbrains.desktop.compose.windows.WindowsWindow.startDragSession],
 * which runs OLE's own blocking `DoDragDrop` loop.
 */
internal class WindowsDragAndDropManager(
    private val nativeWindow: Window,
    private val rootDragAndDropNode: () -> ComposeSceneDragAndDropNode,
) {
    private var previousEffect: DragDropEffect? = null

    /**
     * The formats the current drag carries, captured when it entered.
     *
     * OLE only hands out the data object on enter and on drop; the moves in between get nothing but a
     * position, so the answer to "does this drag carry text?" has to come from a snapshot. Non-null for
     * exactly as long as a drag this window accepted is in flight.
     */
    private var activeFormats: Set<Win32DataFormat>? = null

    fun onDragEnter(
        dataObject: DataObject,
        point: PhysicalPoint,
        allowedEffect: DragDropEffect,
        modifiers: DragDropModifiers,
    ): DragDropEffect {
        val entry = WindowsDragAndDropClipboardEntry(WindowsDragAndDropData.Live(dataObject), allowedEffect)
        val node = rootDragAndDropNode()
        val event = entry.toDragAndDropEvent(point)
        if (!node.acceptDragAndDropTransfer(event)) {
            return DragDropEffect.None
        }
        node.onStarted(event)
        node.onEntered(event)
        // `onMoved` is what lets targets declare the formats they would take, which is what
        // `negotiateEffect` reads — and it is also what establishes the node under the pointer.
        node.onMoved(event)
        val effect = negotiateEffect(entry, allowedEffect, modifiers)
        activeFormats = dataObject.availableFormats()
        previousEffect = effect
        return effect
    }

    fun onDragOver(point: PhysicalPoint, allowedEffect: DragDropEffect, modifiers: DragDropModifiers): DragDropEffect {
        val formats = activeFormats ?: return DragDropEffect.None
        val entry = WindowsDragAndDropClipboardEntry(WindowsDragAndDropData.Formats(formats), allowedEffect)
        val node = rootDragAndDropNode()
        node.onMoved(entry.toDragAndDropEvent(point))
        val effect = negotiateEffect(entry, allowedEffect, modifiers)
        if (effect != previousEffect) {
            node.onChanged(entry.toDragAndDropEvent(point, effect))
            previousEffect = effect
        }
        return effect
    }

    fun onDragLeave() {
        if (activeFormats == null) {
            return
        }
        val entry = WindowsDragAndDropClipboardEntry(WindowsDragAndDropData.Formats(emptySet()))
        val event = DragAndDropEvent(action = null, nativeEvent = entry, positionInRootImpl = Offset.Zero)
        val node = rootDragAndDropNode()
        node.onExited(event)
        node.onEnded(event)
        previousEffect = null
        activeFormats = null
    }

    fun onDrop(dataObject: DataObject, point: PhysicalPoint, allowedEffect: DragDropEffect, modifiers: DragDropModifiers): DragDropEffect {
        val entry = WindowsDragAndDropClipboardEntry(WindowsDragAndDropData.Live(dataObject), allowedEffect)
        val node = rootDragAndDropNode()
        node.onMoved(entry.toDragAndDropEvent(point))
        val effect = negotiateEffect(entry, allowedEffect, modifiers)
        val event = entry.toDragAndDropEvent(point, effect)
        val wasDropConsumed = node.onDrop(event)
        node.onEnded(event)
        previousEffect = null
        activeFormats = null
        return if (wasDropConsumed) effect else DragDropEffect.None
    }

    /**
     * The effect to report back to OLE, which is what decides the cursor the user sees and what the
     * source is told happened.
     *
     * The candidates are the actions a drop target declared it would accept for a format the drag
     * actually carries, narrowed to what the source allows. Held modifiers pick among them — Windows
     * spells copy as Ctrl, move as Shift and link as Alt — and otherwise the target's own first choice
     * wins.
     */
    private fun negotiateEffect(
        entry: WindowsDragAndDropClipboardEntry,
        allowedEffect: DragDropEffect,
        modifiers: DragDropModifiers,
    ): DragDropEffect {
        if (!rootDragAndDropNode().hasEligibleDropTarget) {
            return DragDropEffect.None
        }
        val availableEffects = entry.acceptedFormats()
            .firstNotNullOfOrNull { (format, actions) ->
                actions.filter { allowedEffect.hasFlag(it.toDragDropEffect()) }
                    .ifEmpty { null }
                    .takeIf { entry.containsFormat(format, actions) }
            }
            ?.map { it.toDragDropEffect() }
            ?: return DragDropEffect.None
        val preferred = modifiers.preferredEffect()
        return preferred?.takeIf { it in availableEffects } ?: availableEffects.firstOrNull() ?: DragDropEffect.None
    }

    private fun WindowsDragAndDropClipboardEntry.toDragAndDropEvent(
        point: PhysicalPoint,
        effect: DragDropEffect? = previousEffect,
    ): DragAndDropEvent {
        // Drop positions arrive in physical screen pixels; the scene works in pixels relative to the
        // client area, and on win32 the client area *is* the scene, so mapping to client is the whole
        // conversion — physical pixels are already scene pixels.
        val clientPoint = Screen.mapToClient(nativeWindow, point)
        return DragAndDropEvent(
            action = effect?.toDragAndDropTransferAction(),
            nativeEvent = this,
            positionInRootImpl = Offset(clientPoint.x.toFloat(), clientPoint.y.toFloat()),
        )
    }
}

/**
 * What is known about the drag right now: the data object itself while OLE is lending it out, and
 * otherwise just the set of formats captured when the drag entered.
 */
internal sealed interface WindowsDragAndDropData {
    class Live(val dataObject: DataObject) : WindowsDragAndDropData
    class Formats(val availableFormats: Set<Win32DataFormat>) : WindowsDragAndDropData
}

/**
 * The clipboard behind an in-flight drag, carried as the `nativeEvent` of every [DragAndDropEvent].
 *
 * Reading the data is only possible while OLE is lending the data object out — on enter and on drop —
 * which is exactly when Compose needs it: `onDrop` is where a target takes the payload.
 */
internal class WindowsDragAndDropClipboardEntry(
    private val data: WindowsDragAndDropData,
    private val allowedEffect: DragDropEffect = DragDropEffect.None,
) : DragAndDropClipboardEntry {
    private val acceptedFormats = linkedMapOf<ClipboardFormat<*>, List<DragAndDropTransferAction>>()

    override suspend fun <T : Any> getForFormat(format: ClipboardFormat<T>): List<T> = getForFormatSync(format)

    override fun <T : Any> getForFormatSync(format: ClipboardFormat<T>): List<T> = when (data) {
        is WindowsDragAndDropData.Live -> data.dataObject.readForFormat(format)
        // A move in between: the payload is not on offer, only the promise that it exists.
        is WindowsDragAndDropData.Formats -> emptyList()
    }

    override fun containsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>): Boolean {
        if (!actions.any { allowedEffect.hasFlag(it.toDragDropEffect()) }) {
            return false
        }
        val nativeFormat = format.toWin32DataFormat()
        return when (data) {
            is WindowsDragAndDropData.Formats -> nativeFormat in data.availableFormats
            is WindowsDragAndDropData.Live -> data.dataObject.isFormatAvailableOrFalse(nativeFormat)
        }
    }

    override fun acceptsFormat(format: ClipboardFormat<*>, actions: List<DragAndDropTransferAction>) {
        if (containsFormat(format, actions)) {
            acceptedFormats[format] = actions
        }
    }

    internal fun acceptedFormats(): List<Pair<ClipboardFormat<*>, List<DragAndDropTransferAction>>> =
        acceptedFormats.map { it.key to it.value }
}

private fun DataObject.availableFormats(): Set<Win32DataFormat> = try {
    listItemFormats().toSet()
} catch (exception: DataTransferException) {
    Logger.warn(exception) { "Could not list the formats of the drag" }
    emptySet()
}

private fun DataObject.isFormatAvailableOrFalse(format: Win32DataFormat): Boolean = try {
    isFormatAvailable(format)
} catch (exception: DataTransferException) {
    Logger.warn(exception) { "Could not query whether the drag carries $format" }
    false
}

internal fun DragAndDropTransferAction.toDragDropEffect(): DragDropEffect = when (this) {
    DragAndDropTransferAction.Copy -> DragDropEffect.Copy
    DragAndDropTransferAction.Move -> DragDropEffect.Move
    DragAndDropTransferAction.Link -> DragDropEffect.Link
    else -> DragDropEffect.None
}

internal fun DragDropEffect.toDragAndDropTransferAction(): DragAndDropTransferAction? = when {
    hasFlag(DragDropEffect.Move) -> DragAndDropTransferAction.Move
    hasFlag(DragDropEffect.Copy) -> DragAndDropTransferAction.Copy
    hasFlag(DragDropEffect.Link) -> DragAndDropTransferAction.Link
    else -> null
}

private fun DragDropModifiers.preferredEffect(): DragDropEffect? = when {
    hasFlag(DragDropModifier.Control) && hasFlag(DragDropModifier.Shift) -> DragDropEffect.Link
    hasFlag(DragDropModifier.Alt) -> DragDropEffect.Link
    hasFlag(DragDropModifier.Control) -> DragDropEffect.Copy
    hasFlag(DragDropModifier.Shift) -> DragDropEffect.Move
    else -> null
}
