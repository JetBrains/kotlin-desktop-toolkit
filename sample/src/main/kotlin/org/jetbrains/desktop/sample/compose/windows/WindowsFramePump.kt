package org.jetbrains.desktop.sample.compose.windows

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.yield

/**
 * The frame loop of one window: whoever wants a new frame calls [scheduleFrame], and [onFrame] runs on
 * the event loop thread, at most once per [minFrameIntervalNanos].
 *
 * Requests ride a conflated channel rather than a `WM_PAINT`, which is the whole point of this class.
 * `InvalidateRect` is silently dropped in several window states — hidden before the first show,
 * mid-creation, re-entered dispatch — and a request lost there would wedge the window behind its own
 * deduplication until something else happened to repaint it. A channel cannot lose one.
 *
 * Coalescing is deliberate in both directions: any number of requests before a frame begins produce one
 * frame, and requests that arrive *during* a frame produce exactly one more, because the flag is cleared
 * before [onFrame] runs.
 *
 * Pacing exists because the ANGLE swap does not block on vsync the way Skiko's own loop relies on, so
 * without it a window with a running animation would render as fast as the CPU allows.
 */
internal class WindowsFramePump(
    scope: CoroutineScope,
    private val minFrameIntervalNanos: Long,
    private val onFrame: suspend () -> Unit,
) {
    private val frameChannel = Channel<Unit>(Channel.CONFLATED)

    @Volatile
    private var isFrameScheduled = false

    private var nextFrameDeadlineNanos = System.nanoTime()

    private val job = scope.launch {
        while (true) {
            frameChannel.receive()
            isFrameScheduled = false
            awaitNextFrameSlot()
            onFrame()
            // Back to the dispatcher between frames, so a saturated frame loop cannot starve input.
            yield()
        }
    }

    /**
     * Asks for a frame. Safe to call from any thread: a stale read of the flag costs at most one extra
     * send into a conflated channel, never a lost frame.
     */
    fun scheduleFrame() {
        if (!isFrameScheduled) {
            isFrameScheduled = true
            frameChannel.trySend(Unit)
        }
    }

    fun cancel() {
        job.cancel()
    }

    private suspend fun awaitNextFrameSlot() {
        val waitNanos = nextFrameDeadlineNanos - System.nanoTime()
        if (waitNanos > 0) {
            // Rounded up: delaying a hair too long only shifts the frame, while rounding down would let
            // the loop spin through slots it is not allowed to render in yet.
            delay((waitNanos + 999_999) / 1_000_000)
        }
        nextFrameDeadlineNanos = System.nanoTime() + minFrameIntervalNanos
    }
}
