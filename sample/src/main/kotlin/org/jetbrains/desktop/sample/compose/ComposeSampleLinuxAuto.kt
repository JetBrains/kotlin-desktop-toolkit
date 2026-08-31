package org.jetbrains.desktop.sample.compose

import org.jetbrains.desktop.sample.common.runtimeInfo

/**
 * Runs the sample against whichever Linux backend suits the session — the Wayland one under a Wayland
 * compositor, GTK under X11.
 *
 * The other entry points name their backend and are the way to override this; see [DesktopBackend.default]
 * for what gets read and why `GDK_BACKEND` beats the rest. The composition — [SampleWindows] — is the same
 * one all of them run.
 *
 * Unlike its two siblings this file names no backend at all, which is also why it cannot log through the
 * toolkit: `Logger` is per-backend, and here the backend is the thing being decided. The one line printed
 * before the toolkit exists therefore goes to stdout directly.
 */
fun main() {
    val backend = DesktopBackend.default
    println(runtimeInfo())
    println("Window system: ${currentLinuxWindowSystem()}, chosen backend: $backend")
    initApplication(ComposeApplicationParams(), backend).use { application ->
        runApplication(application) {
            SampleWindows()
        }
    }
}
