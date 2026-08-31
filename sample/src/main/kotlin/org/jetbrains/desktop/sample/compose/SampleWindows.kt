package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp

/**
 * The windows the sample opens.
 *
 * Nothing here is platform-specific — [application] is where the backend was chosen — which is the point:
 * the same composition runs against AppKit, Wayland, GTK and Win32, and the only difference the content
 * sees is what the window reports back about itself.
 */
@Composable
internal fun SampleWindows() {
    AppWindow(title = "Compose Sample", position = DpOffset(80.dp, 80.dp))
    AppWindow(
        title = "Compose Sample 2",
        position = DpOffset(560.dp, 320.dp),
        decoration = WindowDecoration.CustomTitleBar(height = 38.dp, roundedCorners = true),
    )
}

@Composable
private fun AppWindow(title: String, position: DpOffset, decoration: WindowDecoration = WindowDecoration.Decorated) {
    var isWindowShown by remember { mutableStateOf(true) }
    // Every window closes on its own request; the last one to go ends the composition, which is what
    // returns from `runApplication` and lets `main` fall through to `close()`.
    if (isWindowShown) {
        Window(
            title = title,
            size = DpSize(820.dp, 640.dp),
            position = position,
            decoration = decoration,
            onCloseRequest = { isWindowShown = false },
        ) {
            SampleContent()
        }
    }
}
