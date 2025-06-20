package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h
import org.jetbrains.desktop.macos.generated.`dispatcher_main_exec_async$f`
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
            highPriorityQueue.poll()?.let {
                it()
            } ?: queue.poll().invoke()
        }
    }, Arena.global())

    public fun isMainThread(): Boolean {
        return ffiDownCall {
            desktop_macos_h.dispatcher_is_main_thread()
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
        var result: T? = null
        dispatchOnMain(highPriority) {
            result = f()
            latch.countDown()
        }
        latch.await()
        return result!!
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
