package org.jetbrains.desktop.compose

import androidx.compose.runtime.snapshots.ObserverHandle
import androidx.compose.runtime.snapshots.Snapshot
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.cancel
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
    /**
     * Starts flushing, and returns the handle that stops it again. Dispose it once the composition is
     * over and from the event loop thread: until then the write observer is global, so a `mutableStateOf`
     * written anywhere in the process — by a library that has never heard of this toolkit, on a thread of
     * its own — resumes the consumer below through [KdtDispatcher], which needs a live application to
     * resolve a thread to run on.
     */
    fun activateSnapshots(): ObserverHandle {
        val pending = AtomicBoolean(false)
        val channel = Channel<Unit>(capacity = 1)
        // A standalone scope on purpose: this loop only ends when the handle is disposed, so making it a
        // child of the application's scope would stop `runApplication` from ever returning.
        val scope = CoroutineScope(KdtDispatcher)
        scope.launch {
            channel.consumeEach {
                pending.set(false)
                try {
                    Snapshot.sendApplyNotifications()
                } catch (throwable: Throwable) {
                    // A throwing state observer must not end the loop: this coroutine is the only thing
                    // flushing writes made outside a composition, and losing it would silently stop every
                    // later write from invalidating anything.
                    logError(throwable) { "A snapshot apply observer failed" }
                }
            }
        }
        val writeObserver = Snapshot.registerGlobalWriteObserver {
            // Coalesce: one notification is enough no matter how many states were written.
            if (pending.compareAndSet(false, true)) {
                channel.trySend(Unit)
            }
        }
        return ObserverHandle {
            writeObserver.dispose()
            channel.close()
            scope.cancel()
        }
    }
}
