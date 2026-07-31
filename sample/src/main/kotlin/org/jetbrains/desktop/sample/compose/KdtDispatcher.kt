package org.jetbrains.desktop.sample.compose

import kotlinx.coroutines.MainCoroutineDispatcher
import kotlinx.coroutines.Runnable
import org.jetbrains.desktop.macos.GrandCentralDispatch
import kotlin.coroutines.CoroutineContext

/**
 * A coroutine dispatcher that runs everything on the AppKit main thread through Grand Central Dispatch.
 *
 * Compose expects a "UI thread" dispatcher: composition, layout and the scene's frame clock all have to
 * run on the same thread that receives the window events, which for kotlin-desktop-toolkit is the thread
 * captured by [GrandCentralDispatch.startOnMainThread].
 */
sealed class KdtDispatcherBase : MainCoroutineDispatcher() {
    override fun dispatch(context: CoroutineContext, block: Runnable) {
        GrandCentralDispatch.dispatchOnMain {
            block.run()
        }
    }
}

/**
 * Runs the block right away when already on the main thread, instead of always re-dispatching.
 */
object KdtImmediateDispatcher : KdtDispatcherBase() {
    override val immediate: MainCoroutineDispatcher get() = this

    override fun isDispatchNeeded(context: CoroutineContext): Boolean {
        return !GrandCentralDispatch.isMainThread()
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
