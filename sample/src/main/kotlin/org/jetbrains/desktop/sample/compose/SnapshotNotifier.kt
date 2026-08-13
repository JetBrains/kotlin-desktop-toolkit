package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.snapshots.Snapshot
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.launch
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Flushes snapshot writes made outside of composition back into the Compose runtime.
 *
 * Without this, a `mutableStateOf` written from an event handler never invalidates anything, because
 * nobody calls [Snapshot.sendApplyNotifications]. Compose UI ships an internal `GlobalSnapshotManager`
 * that does exactly this, but it is `internal` and therefore unavailable outside of the Compose modules,
 * so the sample carries its own copy built on the public snapshot API.
 */
internal object SnapshotNotifier {
    private val started = AtomicBoolean(false)
    private val pending = AtomicBoolean(false)

    fun ensureStarted() {
        if (!started.compareAndSet(false, true)) {
            return
        }
        val channel = Channel<Unit>(Channel.CONFLATED)
        // A standalone scope on purpose: this loop never finishes, so making it a child of the
        // application's scope would stop `runApplication` from ever returning.
        CoroutineScope(KdtDispatcher).launch {
            for (unused in channel) {
                pending.set(false)
                Snapshot.sendApplyNotifications()
            }
        }
        Snapshot.registerGlobalWriteObserver {
            // Coalesce: one notification is enough no matter how many states were written.
            if (!pending.getAndSet(true)) {
                channel.trySend(Unit)
            }
        }
    }
}
