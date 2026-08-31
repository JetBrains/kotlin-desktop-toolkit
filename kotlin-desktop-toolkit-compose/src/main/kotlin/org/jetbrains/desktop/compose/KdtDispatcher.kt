package org.jetbrains.desktop.compose

import kotlinx.coroutines.MainCoroutineDispatcher
import kotlinx.coroutines.Runnable
import kotlin.coroutines.CoroutineContext

/**
 * A coroutine dispatcher that runs everything on the event loop thread.
 *
 * Compose expects a "UI thread" dispatcher: composition, layout and the scene's frame clock all have to
 * run on the same thread that receives the window events. Which thread that is depends on the platform,
 * so the dispatcher asks the live [ComposeApplication] instead of talking to a platform API — that is
 * also what keeps a single dispatcher working for every backend.
 *
 * This layer always dispatches through here explicitly. Code that instead reaches for `Dispatchers.Main` —
 * a library that knows nothing about this toolkit — needs the separate `kotlin-desktop-toolkit-compose-dispatcher`
 * artifact on the classpath, which registers this dispatcher as the process's main one.
 */
sealed class KdtDispatcherBase : MainCoroutineDispatcher() {
    override fun dispatch(context: CoroutineContext, block: Runnable) {
        currentApplication().invokeOnUiThread {
            block.run()
        }
    }
}

/**
 * Runs the block right away when already on the event loop thread, instead of always re-dispatching.
 */
object KdtImmediateDispatcher : KdtDispatcherBase() {
    override val immediate: MainCoroutineDispatcher get() = this

    override fun isDispatchNeeded(context: CoroutineContext): Boolean {
        return !currentApplication().isUiThread()
    }

    override fun toString(): String {
        return "Dispatchers.KDT.immediate"
    }
}

object KdtDispatcher : KdtDispatcherBase() {
    override val immediate: MainCoroutineDispatcher get() = KdtImmediateDispatcher

    override fun toString(): String {
        return "Dispatchers.KDT"
    }
}
