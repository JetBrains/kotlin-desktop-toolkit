@file:OptIn(ExperimentalComposeUiApi::class, InternalComposeUiApi::class)

package org.jetbrains.desktop.sample.compose

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.draganddrop.dragAndDropSource
import androidx.compose.foundation.draganddrop.dragAndDropTarget
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.InternalComposeUiApi
import androidx.compose.ui.LocalSystemTheme
import androidx.compose.ui.Modifier
import androidx.compose.ui.SystemTheme
import androidx.compose.ui.draganddrop.DragAndDropEvent
import androidx.compose.ui.draganddrop.DragAndDropTarget
import androidx.compose.ui.draganddrop.DragAndDropTransferAction
import androidx.compose.ui.draganddrop.DragAndDropTransferData
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.key.utf16CodePoint
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerIcon
import androidx.compose.ui.input.pointer.PointerKeyboardModifiers
import androidx.compose.ui.input.pointer.isAltPressed
import androidx.compose.ui.input.pointer.isCapsLockOn
import androidx.compose.ui.input.pointer.isCtrlPressed
import androidx.compose.ui.input.pointer.isMetaPressed
import androidx.compose.ui.input.pointer.isShiftPressed
import androidx.compose.ui.input.pointer.pointerHoverIcon
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.isSpecified
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlin.time.Duration.Companion.seconds

/**
 * Content hosted in a kotlin-desktop-toolkit window.
 *
 * Everything here is ordinary Compose reading ordinary Compose state; the point of the sample is that
 * `window.` reads recompose when the platform reports a change, and `window.request*` calls go the other
 * way.
 */
@Composable
fun ComposeWindowScope.SampleContent() {
    // The theme follows the window rather than the application, so a per-window override is visible.
    val colorScheme = when (LocalSystemTheme.current) {
        SystemTheme.Dark -> darkColorScheme()
        else -> lightColorScheme()
    }
    MaterialTheme(colorScheme = colorScheme) {
        Surface(modifier = Modifier.fillMaxSize()) {
            Column(modifier = Modifier.fillMaxSize()) {
                CustomTitleBar()
                WindowBody()
            }
        }
    }
}

/**
 * Draws the window's titlebar whenever the platform is not drawing one.
 *
 * That is either because a custom titlebar was asked for, or — on Wayland, where the compositor decides —
 * because the compositor refused to decorate the window at all; the window reports both as a
 * [WindowDecoration.CustomTitleBar], so the content does not have to know which it is.
 *
 * How much of the bar belongs to the content depends on the platform.
 * [ComposeWindow.customTitleBarInsets] says how much room the platform's own buttons need, and `null`
 * means it drew none at all — in which case a close affordance has to come from here, or the window could
 * not be closed.
 *
 * Moving and maximizing are explicit: an empty area of the bar starts the platform's own move loop on
 * press, and a double click toggles maximized.
 */
@Composable
private fun ComposeWindowScope.CustomTitleBar() {
    val decoration = window.decoration
    if (decoration !is WindowDecoration.CustomTitleBar) {
        return
    }
    val insets = window.customTitleBarInsets
    val (leadingInset, trailingInset) = insets ?: (0.dp to 0.dp)
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(decoration.height)
            .background(MaterialTheme.colorScheme.surfaceVariant),
    ) {
        Box(
            modifier = Modifier
                .weight(1f)
                .fillMaxHeight()
                .pointerInput(window) {
                    // Initial pass: the draggable part of the bar claims the gesture before anything inside
                    // it, which is what a titlebar does. `startInteractiveMove` is only valid while handling
                    // the event, and the event itself is what the platform reads the drag origin from, so it
                    // is passed along.
                    awaitPointerEventScope {
                        while (true) {
                            val event = awaitPointerEvent(PointerEventPass.Initial)
                            window.startInteractiveMove(event)
                        }
                    }
                }
                .pointerInput(window) {
                    detectTapGestures(
                        onDoubleTap = {
                            window.requestPlacement(
                                if (window.placement == WindowPlacement.Maximized) {
                                    WindowPlacement.Floating
                                } else {
                                    WindowPlacement.Maximized
                                },
                            )
                        },
                    )
                }
                .padding(start = leadingInset, end = trailingInset),
            contentAlignment = Alignment.Center,
        ) {
            Text(window.title, style = MaterialTheme.typography.titleSmall)
        }
        if (insets == null) {
            // The platform reserved no room, so it drew no buttons either, and this is the only way out.
            // Deliberately a sibling of the draggable area rather than a child: the move gesture above runs
            // on the initial pass and would otherwise claim the press before the button ever saw it.
            Box(
                modifier = Modifier
                    .fillMaxHeight()
                    .pointerInput(window) {
                        detectTapGestures(onTap = { window.requestClose() })
                    }
                    .padding(horizontal = 14.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text("\u2715", style = MaterialTheme.typography.titleSmall)
            }
        }
    }
}

@Composable
private fun ComposeWindowScope.WindowBody() {
    var clicks by remember { mutableStateOf(0) }
    var text by remember { mutableStateOf("") }
    var lastKey by remember { mutableStateOf("none") }
    var pickedPath by remember { mutableStateOf<String?>(null) }
    val coroutineScope = rememberCoroutineScope()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .onPreviewKeyEvent { event ->
                if (event.type == KeyEventType.KeyDown) {
                    lastKey = "${event.key} (codePoint=${event.utf16CodePoint})"
                }
                false
            }
            .verticalScroll(rememberScrollState())
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(
            text = "Compose in a kotlin-desktop-toolkit window",
            style = MaterialTheme.typography.titleMedium,
        )
        WindowState()

        Button(onClick = { clicks++ }) {
            Text("Clicked $clicks times")
        }

        // The text field exercises the whole input path: key events, and — through the platform's input
        // context — dead keys, accent popovers and IME candidate windows.
        TextField(
            value = text,
            onValueChange = { text = it },
            label = { Text("Type here (dead keys and IME included)") },
        )
        Text("Last key down: $lastKey", style = MaterialTheme.typography.bodySmall)

        WindowControls()

        FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(
                onClick = {
                    coroutineScope.launch {
                        pickedPath = window.showOpenFileDialog(title = "Pick a file").firstOrNull()?.toString()
                    }
                },
            ) {
                Text("Open file…")
            }
            Button(
                onClick = {
                    coroutineScope.launch {
                        pickedPath = window.showSaveFileDialog(nameFieldStringValue = "untitled.txt")?.toString()
                    }
                },
            ) {
                Text("Save file…")
            }
        }
        pickedPath?.let { Text("Picked: $it", style = MaterialTheme.typography.bodySmall) }

        PointerIcons()

        Row(horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.CenterHorizontally)) {
            DragAndDropSource()
            DragAndDropTarget()
        }
    }
}

/** Reads the reactive window properties, so a resize, a move or a focus change shows up here. */
@Composable
private fun ComposeWindowScope.WindowState() {
    val application = application
    val keyboardModifiers = LocalWindowInfo.current.keyboardModifiers
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = "Window ${window.size.width.value.toInt()} x ${window.size.height.value.toInt()} dp " +
                "${window.position.describe()} " +
                "(content ${window.contentSize.width.value.toInt()} x ${window.contentSize.height.value.toInt()})",
            style = MaterialTheme.typography.bodySmall,
        )
        Text(
            text = "${window.placement}, ${if (window.isFocused) "focused" else "not focused"}, " +
                "theme ${window.systemTheme}, density ${window.density.density}",
            style = MaterialTheme.typography.bodySmall,
        )
        Text(
            text = "Screen ${window.screen.name} @ ${window.screen.maximumFramesPerSecond} Hz, " +
                "${application.screens.size} screen(s) attached",
            style = MaterialTheme.typography.bodySmall,
        )
        Text(
            text = "Modifiers held: ${keyboardModifiers.describe()}",
            style = MaterialTheme.typography.bodySmall,
        )
    }
}

@Composable
private fun ComposeWindowScope.WindowControls() {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Button(onClick = { window.requestPlacement(WindowPlacement.Maximized) }) { Text("Maximize") }
        Button(onClick = { window.requestPlacement(WindowPlacement.Floating) }) { Text("Restore") }
        Button(onClick = { window.requestPlacement(WindowPlacement.Fullscreen) }) { Text("Full screen") }
        Button(onClick = { window.requestMinimized(true) }) { Text("Minimize") }
        Button(
            onClick = {
                // Passing null hands the window back to the system theme.
                window.requestSystemTheme(
                    when (window.systemTheme) {
                        SystemTheme.Dark -> SystemTheme.Light
                        else -> SystemTheme.Dark
                    },
                )
            },
        ) {
            Text("Flip theme")
        }
        Button(onClick = { window.requestSystemTheme(null) }) { Text("Follow system") }
        Button(onClick = { application.openUri("https://github.com/JetBrains/kotlin-desktop-toolkit") }) {
            Text("Open URL")
        }
    }
}

/**
 * Cursors. Compose's four built-in icons are recognised by every backend; anything else has to be asked
 * for with a toolkit cursor — `MacOsPointerIcon` or `LinuxPointerIcon`, depending on the platform.
 */
@Composable
private fun PointerIcons() {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        listOf(
            "default" to PointerIcon.Default,
            "text" to PointerIcon.Text,
            "hand" to PointerIcon.Hand,
            "crosshair" to PointerIcon.Crosshair,
        ).forEach { (label, icon) ->
            Text(
                text = label,
                style = MaterialTheme.typography.bodySmall,
                modifier = Modifier
                    .pointerHoverIcon(icon)
                    .background(MaterialTheme.colorScheme.inverseOnSurface)
                    .padding(horizontal = 8.dp, vertical = 4.dp),
            )
        }
    }
}

/** Wayland never tells a client where its window is, and says so with an unspecified offset. */
private fun DpOffset.describe(): String = if (isSpecified) "at ${x.value.toInt()}, ${y.value.toInt()}" else "at an unknown position"

private fun PointerKeyboardModifiers.describe(): String {
    val held = buildList {
        if (isMetaPressed) add("Cmd")
        if (isCtrlPressed) add("Ctrl")
        if (isAltPressed) add("Alt")
        if (isShiftPressed) add("Shift")
        if (isCapsLockOn) add("CapsLock")
    }
    return held.joinToString(" + ").ifEmpty { "none" }
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
                if (showBorder) {
                    border(BorderStroke(3.dp, color = MaterialTheme.colorScheme.onSurfaceVariant))
                } else {
                    this
                }
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
                            dragDropText = event.clipboardEntry
                                .getFirstOrNullForFormatSync(ClipboardFormat.Utf8PlainText)
                                .orEmpty()
                            dragDropTextChangeJob = coroutineScope.launch {
                                delay(2.seconds)
                                dragDropText = "Drop here!"
                            }
                            return true
                        }
                    }
                },
            )
            .background(color = MaterialTheme.colorScheme.inverseOnSurface),
        contentAlignment = Alignment.Center,
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
                    KdtDragAndDropTransferable(
                        ClipEntry(clipboardEntry(ClipboardItem("Hello from KDT!", ClipboardFormat.Utf8PlainText))),
                    ),
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
        contentAlignment = Alignment.Center,
    ) {
        Text(dragSourceText)
    }
}
