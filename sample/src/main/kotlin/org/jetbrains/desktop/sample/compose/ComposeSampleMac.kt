package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.sample.common.runtimeInfo

/**
 * Runs Compose content inside kotlin-desktop-toolkit windows.
 *
 * [initApplication] boots the native application and [runApplication] opens the composition; from there
 * on it is ordinary Compose, and [Window] is the bridge to a native window.
 *
 * Nothing below is macOS-specific — the platform implementation is chosen inside [initApplication] — so
 * this file is what a cross-platform entry point would look like once another backend exists.
 */
fun main() {
    Logger.info { runtimeInfo() }
    val params = ComposeApplicationParams(
        onOpenUris = { uris -> Logger.info { "Asked to open $uris" } },
    )
    initApplication(params).use { application ->
        runApplication(application) {
            AppWindow(title = "Compose Sample", position = DpOffset(80.dp, 80.dp))
            AppWindow(
                title = "Compose Sample 2",
                position = DpOffset(560.dp, 320.dp),
                decoration = WindowDecoration.CustomTitleBar(height = 38.dp, roundedCorners = true),
            )
        }
    }
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
