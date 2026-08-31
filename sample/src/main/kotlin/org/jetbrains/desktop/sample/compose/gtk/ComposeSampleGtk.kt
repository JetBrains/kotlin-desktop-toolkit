package org.jetbrains.desktop.sample.compose.gtk

import org.jetbrains.desktop.gtk.Logger
import org.jetbrains.desktop.sample.common.runtimeInfo
import org.jetbrains.desktop.sample.compose.ComposeApplicationParams
import org.jetbrains.desktop.sample.compose.DesktopBackend
import org.jetbrains.desktop.sample.compose.SampleWindows
import org.jetbrains.desktop.sample.compose.initApplication
import org.jetbrains.desktop.sample.compose.runApplication

/**
 * Runs Compose content inside kotlin-desktop-toolkit windows on GTK.
 *
 * The twin of the macOS and Wayland entry points, and deliberately as thin: the whole difference between
 * the backends is behind `initApplication`, and the composition — [SampleWindows] — is shared verbatim.
 *
 * GTK is the toolkit's other Linux backend and runs on X11 as well as Wayland. This entry point pins it,
 * which is what makes it the way to run GTK under a Wayland session; [DesktopBackend.default] would prefer
 * the Wayland backend there, and `ComposeSampleLinuxAuto` is the entry point that lets it choose.
 */
fun main() {
    Logger.info { runtimeInfo() }
    initApplication(ComposeApplicationParams(), DesktopBackend.Gtk).use { application ->
        runApplication(application) {
            SampleWindows()
        }
    }
}
