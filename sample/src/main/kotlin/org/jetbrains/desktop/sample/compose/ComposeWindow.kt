package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.awaitCancellation
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.LogicalSize

/** Receiver of a window's content, so the content can look at the window hosting it. */
interface ComposeWindowScope {
    val window: ComposeWindow
}

interface ComposeWindow : AutoCloseable {
    /** Size of the whole window, titlebar included. */
    val size: DpSize

    /** Size of the area the content is drawn into. */
    val contentSize: DpSize

    /** Whether this is the application's main window. */
    val isActive: Boolean

    /** Whether this window receives keyboard input. */
    val isKey: Boolean

    fun setContent(content: @Composable () -> Unit)
}

/**
 * Opens a native window for as long as this composable stays in the composition.
 *
 * Closing it is up to the caller: [onCloseRequested] fires when the user clicks the close button, and
 * the window goes away once the caller stops emitting this composable.
 */
@Composable
fun Window(
    title: String = "Untitled",
    size: DpSize = DpSize(800.dp, 600.dp),
    position: DpOffset = DpOffset.Zero,
    onCloseRequested: () -> Unit,
    content: @Composable ComposeWindowScope.() -> Unit,
) {
    val application = LocalComposeApplication.current
    // The callback can change between recompositions while the window itself must not be recreated.
    val currentOnCloseRequested = rememberUpdatedState(onCloseRequested)
    val composeWindow = remember {
        application.createWindow(
            title = title,
            origin = LogicalPoint(position.x.value.toDouble(), position.y.value.toDouble()),
            size = LogicalSize(size.width.value.toDouble(), size.height.value.toDouble()),
            onCloseRequested = { currentOnCloseRequested.value.invoke() },
        )
    }
    DisposableEffect(Unit) {
        onDispose {
            composeWindow.close()
        }
    }
    val windowScope = remember {
        object : ComposeWindowScope {
            override val window: ComposeWindow = composeWindow
        }
    }
    // Setting content from an effect that never completes keeps the Recomposer from deciding the
    // composition is finished while this window is still on screen.
    LaunchedEffect(Unit) {
        composeWindow.setContent {
            windowScope.content()
        }
        awaitCancellation()
    }
}
