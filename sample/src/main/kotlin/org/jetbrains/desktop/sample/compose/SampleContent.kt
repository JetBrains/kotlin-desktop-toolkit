@file:OptIn(ExperimentalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.draganddrop.dragAndDropSource
import androidx.compose.foundation.draganddrop.dragAndDropTarget
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTarget
import androidx.compose.ui.draganddrop.DragAndDropTransferAction
import androidx.compose.ui.draganddrop.DragAndDropTransferData
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.key.utf16CodePoint
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.time.Duration.Companion.seconds

/**
 * Content hosted in a kotlin-desktop-toolkit window.
 *
 * The button exercises pointer events, the text field exercises key events, and the reported size shows
 * that resizes reach the composition.
 */
@Composable
fun ComposeWindowScope.SampleContent() {
    MaterialTheme(colorScheme = darkColorScheme()) {
        var clicks by remember { mutableStateOf(0) }
        var text by remember { mutableStateOf("") }
        var lastKey by remember { mutableStateOf("none") }
        // Surface paints the theme's background and sets the matching content colour for everything
        // inside it; without one, text falls back to black on whatever the frame was cleared to.
        Surface(modifier = Modifier.fillMaxSize()) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .onPreviewKeyEvent { event ->
                        if (event.type == KeyEventType.KeyDown) {
                            lastKey = "${event.key} (codePoint=${event.utf16CodePoint})"
                        }
                        false
                    }
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterVertically),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text(
                    text = "Compose in a kotlin-desktop-toolkit window",
                    style = MaterialTheme.typography.titleMedium,
                )
                Text(
                    text = "Content size: ${window.contentSize.width.value.toInt()} x ${window.contentSize.height.value.toInt()} dp",
                    style = MaterialTheme.typography.bodySmall,
                )
                Button(onClick = { clicks++ }) {
                    Text("Clicked $clicks times")
                }
                TextField(
                    value = text,
                    onValueChange = { text = it },
                    label = { Text("Type here") },
                )
                Text(
                    text = "Last key down: $lastKey",
                    style = MaterialTheme.typography.bodySmall,
                )

                Row(horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterHorizontally), ) {
                    DragAndDropSource()
                    DragAndDropTarget()
                }
            }
        }
    }
}

@Composable
private fun DragAndDropTarget() {
    var dragDropText by remember { mutableStateOf("Drop here!") }
    val coroutineScope = rememberCoroutineScope()
    var dragDropTextChangeJob: Job? = null
    var showBorder by remember { mutableStateOf(false) }
    Box(
        Modifier
            .size(100.dp)
            .run {
                if (showBorder) border(BorderStroke(3.dp, color = MaterialTheme.colorScheme.onSurfaceVariant))
                else this
            }
            .dragAndDropTarget(
                shouldStartDragAndDrop = { true },
                remember {
                    object : DragAndDropTarget {
                        override fun onMoved(event: DragAndDropEvent) {
                            // `acceptsFormat` allows for dynamic cursor control.
                            // Not possible in Compose AWT - https://youtrack.jetbrains.com/issue/CMP-7806
                            event.acceptsFormat(ClipboardFormat.Utf8PlainText, listOf(DragAndDropTransferAction.Move))
                            super.onMoved(event)
                        }

                        override fun onEntered(event: DragAndDropEvent) {
                            showBorder = true
                        }

                        override fun onExited(event: DragAndDropEvent) {
                            showBorder = false
                        }

                        override fun onEnded(event: DragAndDropEvent) {
                            showBorder = false
                        }

                        override fun onDrop(event: DragAndDropEvent): Boolean {
                            dragDropTextChangeJob?.cancel()
                            dragDropText = event.clipboardEntry.getFirstOrNullForFormatSync(ClipboardFormat.Utf8PlainText).orEmpty()
                            dragDropTextChangeJob = coroutineScope.launch {
                                delay(2.seconds)
                                dragDropText = "Drop here!"
                            }
                            return true
                        }
                    }
                })
            .background(color = MaterialTheme.colorScheme.inverseOnSurface),
        contentAlignment = Alignment.Center
    ) {
        Text(dragDropText, textAlign = TextAlign.Center)
    }
}

@Composable
private fun DragAndDropSource() {
    var dragSourceText by remember { mutableStateOf("Drag me!") }
    val coroutineScope = rememberCoroutineScope()
    var dragSourceTextChangeJob: Job? = null
    Box(
        Modifier
            .size(100.dp)
            .dragAndDropSource {
                DragAndDropTransferData(
                    KdtDragAndDropTransferable(ClipEntry(clipboardEntry(ClipboardItem("Hello from KDT!", ClipboardFormat.Utf8PlainText)))),
                    supportedActions = listOf(DragAndDropTransferAction.Move),
                ) {
                    dragSourceTextChangeJob?.cancel()
                    dragSourceText = "Dropped!"
                    dragSourceTextChangeJob = coroutineScope.launch {
                        delay(2.seconds)
                        dragSourceText = "Drag me!"
                    }
                }
            }
            .background(color = MaterialTheme.colorScheme.inverseOnSurface),
        contentAlignment = Alignment.Center
    ) {
        Text(dragSourceText)
    }
}
