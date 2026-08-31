package org.jetbrains.desktop.sample.compose.windows

import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.desktop.sample.compose.ComposeApplicationParams
import org.jetbrains.desktop.sample.compose.SampleWindows
import org.jetbrains.desktop.sample.compose.initApplication
import org.jetbrains.desktop.sample.compose.runApplication
import org.jetbrains.desktop.win32.Logger

/**
 * Runs Compose content inside kotlin-desktop-toolkit windows on Windows.
 *
 * `initApplication` boots the native application and `runApplication` opens the composition; from there on
 * it is ordinary Compose, and `Window` is the bridge to a native window.
 *
 * Only the logger is win32-specific here — the toolkit deliberately does not unify its per-platform APIs,
 * so even logging is per-backend. Everything the composition does lives in [SampleWindows], which the macOS
 * and Linux entry points run unchanged.
 */
fun main() {
    Logger.info { runtimeInfo() }
    initApplication(ComposeApplicationParams()).use { application ->
        runApplication(application) {
            SampleWindows()
        }
    }
}
