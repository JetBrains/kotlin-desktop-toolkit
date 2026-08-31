package org.jetbrains.desktop.sample.compose

import org.jetbrains.desktop.gtk.Logger as GtkLogger
import org.jetbrains.desktop.linux.Logger as LinuxLogger
import org.jetbrains.desktop.macos.Logger as MacOsLogger
import org.jetbrains.desktop.win32.Logger as WindowsLogger

/**
 * Logging that does not require the caller to know which backend it is talking to.
 *
 * The toolkit deliberately does not unify its per-platform APIs, so even `Logger` is per-backend and there
 * are four unrelated objects with the same shape. Callers above this layer have a [DesktopBackend] rather
 * than a platform package, so they need this to be able to log at all.
 *
 * Only the branch that is actually taken ever gets loaded, which is what keeps a reference to all four
 * platforms' classes harmless: each `Logger` pulls in its own native library on first touch.
 */
internal fun logInfo(backend: DesktopBackend, message: () -> String) {
    when (backend) {
        DesktopBackend.MacOs -> MacOsLogger.info(message)
        DesktopBackend.Wayland -> LinuxLogger.info(message)
        DesktopBackend.Gtk -> GtkLogger.info(message)
        DesktopBackend.Windows -> WindowsLogger.info(message)
    }
}
