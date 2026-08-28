package org.jetbrains.desktop.sample.compose

import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.draganddrop.DragAndDropTransferData
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.platform.PlatformDragAndDropManager
import androidx.compose.ui.platform.PlatformDragAndDropSource

/**
 * Turns Compose's request to start a drag into a platform drag session.
 *
 * Platform-agnostic on purpose: everything platform-specific lives behind [DragAndDropSourceWindow],
 * so this class is shared by every backend.
 */
@OptIn(InternalComposeUiApi::class)
internal class KdtDragAndDropManager(private val window: DragAndDropSourceWindow) : PlatformDragAndDropManager {
    override val isRequestDragAndDropTransferRequired: Boolean = true

    override fun requestDragAndDropTransfer(source: PlatformDragAndDropSource, offset: Offset) {
        var isTransferStarted = false
        val startTransferScope = object : PlatformDragAndDropSource.StartTransferScope {
            override fun startDragAndDropTransfer(
                transferData: DragAndDropTransferData,
                decorationSize: Size,
                drawDragDecoration: DrawScope.() -> Unit,
            ): Boolean {
                isTransferStarted = true
                window.startDragSession(
                    offset,
                    transferData,
                    decorationSize,
                    drawDragDecoration,
                )
                return isTransferStarted
            }
        }
        with(source) {
            startTransferScope.startDragAndDropTransfer(offset) {
                isTransferStarted
            }
        }
    }
}
