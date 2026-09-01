package org.jetbrains.desktop.compose.macos

import org.jetbrains.desktop.macos.Logger
import java.util.concurrent.atomic.AtomicReference

/**
 * Decides, on every display link tick, whether to produce a frame.
 *
 * The point of the class is a single invariant: after each tick, either a frame has been handed to
 * [presentAsync], or the "frame wanted" flag is set *and* the in-flight latch is clear so the next tick
 * can try again. Getting that wrong either drops frames forever (a latch that stays held after an
 * exception) or queues frames without bound.
 *
 * [onDisplayLinkTick] runs on the display link's own thread; the frame body is posted to the event loop
 * thread through [dispatchOnMain], because recording a frame touches the scene.
 *
 * Generic over the picture type so the state machine can be exercised without Skia or Metal.
 */
internal class DisplayLinkFramePump<P : AutoCloseable>(
    private val isDisposed: () -> Boolean,
    private val isFrameRequested: () -> Boolean,
    private val setFrameRequested: (Boolean) -> Unit,
    private val dispatchOnMain: (block: () -> Unit) -> Unit,
    private val recordFrame: () -> P?,
    private val presentAsync: (picture: P, onComplete: () -> Unit) -> Unit,
) {
    /** Non-null while a frame is being recorded or presented. Holding it is what serialises frames. */
    private val frameInFlight = AtomicReference<Any?>(null)

    val isFrameInFlight: Boolean get() = frameInFlight.get() != null

    fun onDisplayLinkTick() {
        val token = Any()
        if (isDisposed() || !isFrameRequested() || !frameInFlight.compareAndSet(null, token)) {
            return
        }
        dispatchOnMain {
            setFrameRequested(false)
            try {
                val picture = recordFrame()
                if (picture == null) {
                    // Nothing to show yet (the window is gone, or the view has no drawable size).
                    retry(token)
                    return@dispatchOnMain
                }
                try {
                    presentAsync(picture) {
                        picture.close()
                        frameInFlight.set(null)
                    }
                } catch (throwable: Throwable) {
                    Logger.error(throwable) { "Could not schedule frame presentation" }
                    picture.close()
                    retry(token)
                }
            } catch (throwable: Throwable) {
                Logger.error(throwable) { "Could not record frame" }
                retry(token)
            }
        }
    }

    /**
     * Releases the latch this tick took and asks for the frame again.
     *
     * Compare-and-set rather than a plain clear: a synchronous repaint may have taken over in the
     * meantime, and clearing unconditionally would drop *its* latch instead of ours.
     */
    private fun retry(token: Any) {
        setFrameRequested(true)
        frameInFlight.compareAndSet(token, null)
    }
}
