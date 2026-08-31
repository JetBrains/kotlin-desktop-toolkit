package org.jetbrains.desktop.compose

import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.MotionDurationScale
import androidx.compose.ui.platform.InfiniteAnimationPolicy
import kotlinx.coroutines.flow.first
import kotlin.coroutines.CoroutineContext

/**
 * Desktop application-level content is not synchronized to a display. When there is no native window,
 * finite animations should complete in a single frame and infinite animations should suspend until a
 * window is attached again.
 *
 * This preserves non-visual side effects and the recomposition needed to materialize the next window
 * without letting a window-less composition spin the frame clock as fast as possible.
 */
internal fun noWindowAnimationCoroutineContext(hasNativeWindows: () -> Boolean): CoroutineContext {
    return NoWindowMotionDurationScale(hasNativeWindows) + NoWindowInfiniteAnimationPolicy(hasNativeWindows)
}

private class NoWindowMotionDurationScale(private val hasNativeWindows: () -> Boolean) : MotionDurationScale {
    override val scaleFactor: Float
        get() = if (hasNativeWindows()) 1f else 0f
}

private class NoWindowInfiniteAnimationPolicy(private val hasNativeWindows: () -> Boolean) : InfiniteAnimationPolicy {
    override suspend fun <R> onInfiniteOperation(block: suspend () -> R): R {
        if (!hasNativeWindows()) {
            snapshotFlow { hasNativeWindows() }.first { it }
        }
        return block()
    }
}
