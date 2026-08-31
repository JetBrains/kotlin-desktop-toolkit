package org.jetbrains.desktop.compose

import java.util.concurrent.ConcurrentHashMap

/** A window the quit sequence can ask to close. Implemented by every backend's window. */
internal interface QuitTarget {
    fun requestCloseFromQuit()
}

/**
 * The structured quit, shared by all four backends.
 *
 * Nothing here terminates anything: the quit handlers get their veto, every open window is asked to close
 * through its own `onCloseRequest`, and the composition does the rest — once it stops emitting those
 * windows `runApplication` returns and `application` closes the toolkit on the way out. With no window
 * left to make that decision, [requestApplicationExit] makes it instead.
 *
 * This is why no backend hands its native side a "yes, terminate": AppKit's `TerminateNow` halts the
 * process where it stands, and GTK's `gtk_app.quit()` drops the GLib queue the Compose dispatcher runs on.
 */
internal class ApplicationQuit(
    private val backend: DesktopBackend,
    private val openWindows: () -> List<QuitTarget>,
) {
    private val quitHandlers = ConcurrentHashMap<String, () -> Boolean>()

    /**
     * Guards one round against being re-entered from a window's own `onCloseRequest`, which runs inline
     * below. Always cleared again, so a quit the application declined leaves nothing behind.
     */
    @Volatile
    private var structuredQuitInProgress = false

    fun putQuitHandler(id: String, isSafeToQuit: () -> Boolean) {
        quitHandlers[id] = isSafeToQuit
    }

    fun removeQuitHandler(id: String) {
        quitHandlers.remove(id)
    }

    /** Must be called on the event loop thread. */
    fun requestStructuredQuit() {
        // Every handler is consulted, not just up to the first veto: a handler may be showing a dialog and
        // needs to know a quit was attempted even if another handler already said no.
        val isSafeToQuit = quitHandlers.values.fold(true) { acc, handler -> handler() and acc }
        if (!isSafeToQuit || structuredQuitInProgress) {
            return
        }
        val windows = openWindows()
        if (windows.isEmpty()) {
            requestApplicationExit(backend)
            return
        }
        logInfo(backend) { "Quit approved, asking ${windows.size} window(s) to close" }
        structuredQuitInProgress = true
        try {
            windows.forEach { it.requestCloseFromQuit() }
        } finally {
            structuredQuitInProgress = false
        }
    }
}
