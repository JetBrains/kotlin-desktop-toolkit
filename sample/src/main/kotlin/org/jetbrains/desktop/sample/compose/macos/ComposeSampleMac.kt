package org.jetbrains.desktop.sample.compose.macos

import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.desktop.sample.compose.ComposeApplicationParams
import org.jetbrains.desktop.sample.compose.SampleWindows
import org.jetbrains.desktop.sample.compose.initApplication
import org.jetbrains.desktop.sample.compose.runApplication

/**
 * Runs Compose content inside kotlin-desktop-toolkit windows on macOS.
 *
 * `initApplication` boots the native application and `runApplication` opens the composition; from there on
 * it is ordinary Compose, and `Window` is the bridge to a native window.
 *
 * Only the logger is macOS-specific here — the toolkit deliberately does not unify its per-platform APIs,
 * so even logging is per-backend. Everything the composition does lives in [SampleWindows], which the Linux
 * entry point runs unchanged.
 */
fun main() {
    Logger.info { runtimeInfo() }
    val params = ComposeApplicationParams(
        onOpenUris = { uris -> Logger.info { "Asked to open $uris" } },
    )
    initApplication(params).use { application ->
        runApplication(application) {
            SampleWindows()
        }
    }
}
