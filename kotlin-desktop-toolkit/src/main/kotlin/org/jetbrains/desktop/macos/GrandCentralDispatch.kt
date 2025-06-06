package org.jetbrains.desktop.macos

import org.jetbrains.desktop.macos.generated.desktop_macos_h
import org.jetbrains.desktop.macos.generated.`dispatcher_main_exec_async$f`
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CountDownLatch

public object GrandCentralDispatch {
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
            if (highPriority) {
                highPriorityQueue.add(f)
            } else {
                queue.add(f)
            }
            desktop_macos_h.dispatcher_main_exec_async(callback)
        }
    }

    public fun <T> dispatchOnMainSync(highPriority: Boolean = false, f: () -> T): T {
        val latch = CountDownLatch(1)
        var result: T? = null
        dispatchOnMain(highPriority) {
            result = f()
            latch.countDown()
        }
        latch.await()
        return result!!
    }
}
