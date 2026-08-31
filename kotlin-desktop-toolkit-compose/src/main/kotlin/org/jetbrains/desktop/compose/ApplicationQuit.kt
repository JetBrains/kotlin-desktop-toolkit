package org.jetbrains.desktop.compose

import java.util.concurrent.ConcurrentHashMap

/**
 * A window the quit sequence can ask to close.
 *
 * Not on [ComposeWindow] itself because the reason a close carries is not the content's to pick: only the
 * backend that owns the window may report a [WindowCloseRequestReason.ApplicationQuit], and content that
 * wants to close a window has [ComposeWindow.requestClose].
 */
internal interface QuitTarget {
    /** Asks this window's owner to close it because the application is quitting. */
    fun requestCloseFromQuit()
}

/**
 * The quit sequence, shared by all four backends.
 *
 * Quitting is a negotiation rather than an action, and nothing here terminates anything. Every registered
 * handler gets a veto; if none of them takes it, every open window is asked to close through its own
 * `onCloseRequest`, which is where the application makes the decision. What actually ends the process is
 * the composition: once it stops emitting those windows, `runApplication` returns and `application` closes
 * the toolkit on the way out. An application with no window left to negotiate with has nobody to make that
 * decision, so the sequence makes it directly, through [requestApplicationExit].
 *
 * Driving the exit through the composition on every platform is deliberate, and it is why no backend
 * hands its native side a "yes, terminate" answer: AppKit's `TerminateNow` halts the process where it
 * stands, and GTK's `gtk_app.quit()` tears down the GLib main loop with the Compose dispatcher's queue
 * still on it — both of which would cut the composition off mid-shutdown rather than let it unwind.
 */
internal class ApplicationQuit(
    private val backend: DesktopBackend,
    private val openWindows: () -> List<QuitTarget>,
) {
    private val handlers = ConcurrentHashMap<String, () -> Boolean>()

    /**
     * Guards one round of [request] against being re-entered from a window's own `onCloseRequest`, which
     * runs inline below and is free to call back into [ComposeApplication.quit].
     *
     * Scoped to the round and always cleared again, so a quit the application declined leaves nothing
     * behind: the next one negotiates from scratch, the way a second press of Quit should.
     */
    @Volatile
    private var isInProgress = false

    fun putHandler(id: String, isSafeToQuit: () -> Boolean) {
        handlers[id] = isSafeToQuit
    }

    fun removeHandler(id: String) {
        handlers.remove(id)
    }

    /** Runs the quit sequence. Must be called on the event loop thread. */
    fun request() {
        // Every handler is consulted, not just up to the first veto: a handler may be showing a dialog and
        // needs to know a quit was attempted even if another handler already said no.
        val isSafeToQuit = handlers.values.fold(true) { acc, handler -> handler() and acc }
        if (!isSafeToQuit) {
            logInfo(backend) { "Quit vetoed by a quit handler" }
            return
        }
        if (isInProgress) {
            return
        }
        val windows = openWindows()
        if (windows.isEmpty()) {
            logInfo(backend) { "Quit approved with no window open, ending the composition" }
            requestApplicationExit(backend)
            return
        }
        logInfo(backend) { "Quit approved, asking ${windows.size} window(s) to close" }
        isInProgress = true
        try {
            windows.forEach { it.requestCloseFromQuit() }
        } finally {
            isInProgress = false
        }
    }
}
