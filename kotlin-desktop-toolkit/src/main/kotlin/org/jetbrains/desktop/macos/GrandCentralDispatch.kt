package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h
import org.jetbrains.desktop.macos.generated.`dispatcher_main_exec_async$f`
import org.jetbrains.desktop.macos.generated.`dispatcher_start_on_main_thread$f`
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CountDownLatch
import kotlin.concurrent.Volatile

public object GrandCentralDispatch : AutoCloseable {
    @Volatile
    private var isClosed = false
    private val highPriorityQueue = ConcurrentLinkedQueue<() -> Unit>()
    private val queue = ConcurrentLinkedQueue<() -> Unit>()
    private val callback: MemorySegment = `dispatcher_main_exec_async$f`.allocate({
        ffiUpCall {
            if (!isClosed) {
                val f = highPriorityQueue.poll() ?: queue.poll()
                f!!.invoke()
            }
        }
    }, Arena.global())

    public fun isMainThread(): Boolean {
        return ffiDownCall {
            desktop_macos_h.dispatcher_is_main_thread()
        }
    }

    /**
     * On MacOS access to NSApplication and other UI related classes are only allowed from the Main thread.
     * Usually you can dispatch on Main with `dispatchOnMain { ... }` function, which sends the task to the Main.
     * Under the hood `dispatchOnMain` uses application main queue, see: https://developer.apple.com/documentation/dispatch/dispatch_get_main_queue?language=objc
     * But for initial capture of Main thread, to start Application event loop you need to use this `startOnMainThread` function.
     * It relies on lower level mechanism, which allows reentrancy, opposite to `dispatchOnMain`.
     */
    public fun startOnMainThread(body: () -> Unit) {
        if (isMainThread()) {
            body()
        } else {
            Arena.ofShared().use { arena ->
                desktop_macos_h.dispatcher_start_on_main_thread(
                    `dispatcher_start_on_main_thread$f`.allocate({
                        ffiUpCall {
                            Thread.currentThread().name = "AppKit Main Thread"
                            body()
                        }
                    }, arena),
                )
            }
        }
    }

    public fun dispatchOnMain(highPriority: Boolean = false, f: () -> Unit) {
        ffiDownCall {
            checkIsNotClosed()
            if (highPriority) {
                highPriorityQueue.add(f)
            } else {
                queue.add(f)
            }
            desktop_macos_h.dispatcher_main_exec_async(callback)
        }
    }

    public fun <T> dispatchOnMainSync(highPriority: Boolean = false, f: () -> T): T {
        checkIsNotClosed()
        val latch = CountDownLatch(1)
        var result: Result<T>? = null
        dispatchOnMain(highPriority) {
            result = runCatching(f)
            latch.countDown()
        }
        latch.await()
        return result!!.getOrThrow()
    }

    private fun checkIsNotClosed() {
        assert(!isClosed) { "GrandCentralDispatch is closed" }
    }

    /**
     * Should be called from AppKit thread, usually after `Application.stopEventLoop`
     * it ensures that all scheduled tasks are executed, and mark the dispatcher as closed.
     */
    override fun close() {
        isClosed = true
        var task = highPriorityQueue.poll()
        while (task != null) {
            task()
            task = highPriorityQueue.poll()
        }
        task = queue.poll()
        while (task != null) {
            task()
            task = queue.poll()
        }
    }
}

@JvmInline
public value class QualityOfService internal constructor(internal val x: Long) {
    public companion object {
        public val UserInteractive: QualityOfService = QualityOfService(0x21L)
        public val UserInitiated: QualityOfService = QualityOfService(0x19L)
        public val Utility: QualityOfService = QualityOfService(0x11L)
        public val Background: QualityOfService = QualityOfService(0x09)
        public val Default: QualityOfService = QualityOfService(-1)
    }
}

public fun setQualityOfServiceForCurrentThread(qualityOfService: QualityOfService) {
    ffiDownCall {
        desktop_macos_h.set_qos_for_current_thread(qualityOfService.x)
    }
}

public fun withAutoReleasePool(f: () -> Unit) {
    val pool = ffiDownCall {
        desktop_macos_h.push_autorelease_pool()
    }
    try {
        f()
    } finally {
        ffiDownCall {
            desktop_macos_h.pop_autorelease_pool(pool)
        }
    }
}
