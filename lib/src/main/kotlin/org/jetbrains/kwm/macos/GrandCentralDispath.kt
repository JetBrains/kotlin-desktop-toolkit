package org.jetbrains.kwm.macos

import org.jetbrains.kwm.IGrandCentralDispatch
import org.jetbrains.kwm.macos.generated.`dispatcher_main_exec_async$f`
import org.jetbrains.kwm.macos.generated.kwm_macos_h
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CountDownLatch

object GrandCentralDispatch: IGrandCentralDispatch {
    private val queue = ConcurrentLinkedQueue<() -> Unit>()
    private val callback: MemorySegment = `dispatcher_main_exec_async$f`.allocate({
        queue.poll().invoke()
    }, Arena.global())

    override fun isMainThread(): Boolean {
        return kwm_macos_h.dispatcher_is_main_thread()
    }

    override fun dispatchOnMain(f: () -> Unit) {
        queue.add(f)
        kwm_macos_h.dispatcher_main_exec_async(callback)
    }

    fun <T> dispatchOnMainSync(f: () -> T): T {
        val latch = CountDownLatch(1)
        var result: T? = null
        dispatchOnMain {
            result = f()
            latch.countDown()
        }
        latch.await()
        return result!!
    }
}