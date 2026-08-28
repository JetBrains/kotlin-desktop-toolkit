package org.jetbrains.desktop.sample.compose

import androidx.compose.runtime.snapshots.Snapshot
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.channels.consumeEach
import kotlinx.coroutines.launch
import java.util.concurrent.atomic.AtomicBoolean

/**
 * Flushes snapshot writes made outside of composition back into the Compose runtime, on the event loop
 * thread.
 *
 * A `mutableStateOf` written from an event handler invalidates nothing until somebody calls
 * [Snapshot.sendApplyNotifications]. Compose UI has an internal `GlobalSnapshotManager` that does this,
 * which every `ComposeScene` starts on construction — but on desktop it runs on Skiko's
 * `MainUIDispatcher`, which is the AWT event dispatch thread, a thread this application otherwise never
 * touches. Snapshot state is written from the toolkit's event loop thread, so that is where it should be
 * applied: on the same thread, in the same turn, before the next frame is recorded.
 *
 * It is `internal` and cannot be replaced, so Compose's own manager keeps running alongside this one.
 * That is harmless — [Snapshot.sendApplyNotifications] is safe to call from anywhere and does nothing
 * when there is nothing pending — but it does mean this class is what makes the timing predictable
 * rather than what makes notification happen at all.
 */
internal object KdtSnapshotManager {
    private val started = AtomicBoolean(false)
    private val pending = AtomicBoolean(false)

    fun ensureStarted() {
        if (!started.compareAndSet(false, true)) {
            return
        }
        val channel = Channel<Unit>(capacity = 1)
        // A standalone scope on purpose: this loop never finishes, so making it a child of the
        // application's scope would stop `runApplication` from ever returning.
        CoroutineScope(KdtDispatcher).launch {
            channel.consumeEach {
                pending.set(false)
                Snapshot.sendApplyNotifications()
            }
        }
        Snapshot.registerGlobalWriteObserver {
            // Coalesce: one notification is enough no matter how many states were written.
            if (pending.compareAndSet(false, true)) {
                channel.trySend(Unit)
            }
        }
    }
}
