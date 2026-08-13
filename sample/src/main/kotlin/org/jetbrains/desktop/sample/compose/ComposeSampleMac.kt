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
 */
fun main() {
    Logger.info { runtimeInfo() }
    initApplication().use { application ->
        runApplication(application) {
            AppWindow(title = "Compose Sample", position = DpOffset(80.dp, 80.dp))
            AppWindow(title = "Compose Sample 2", position = DpOffset(560.dp, 320.dp))
        }
    }
}

@Composable
private fun AppWindow(title: String, position: DpOffset) {
    var isWindowShown by remember { mutableStateOf(true) }
    if (isWindowShown) {
        Window(
            title = title,
            size = DpSize(800.dp, 600.dp),
            position = position,
            onCloseRequested = { isWindowShown = false },
        ) {
            SampleContent()
        }
    }
}
