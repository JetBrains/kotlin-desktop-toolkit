package org.jetbrains.desktop.sample.compose.linux

import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.desktop.sample.compose.ComposeApplicationParams
import org.jetbrains.desktop.sample.compose.SampleWindows
import org.jetbrains.desktop.sample.compose.initApplication
import org.jetbrains.desktop.sample.compose.runApplication

/**
 * Runs Compose content inside kotlin-desktop-toolkit windows on Linux (Wayland).
 *
 * The twin of the macOS entry point, and deliberately as thin: the whole difference between the two
 * backends is behind `initApplication`, and the composition — [SampleWindows] — is shared verbatim.
 *
 * Names its backend, so it requires a Wayland session. `ComposeSampleGtk` is the same sample against the
 * toolkit's GTK backend, and `ComposeSampleLinuxAuto` picks between the two to suit the session.
 */
fun main() {
    Logger.info { runtimeInfo() }
    initApplication(ComposeApplicationParams()).use { application ->
        runApplication(application) {
            SampleWindows()
        }
    }
}
