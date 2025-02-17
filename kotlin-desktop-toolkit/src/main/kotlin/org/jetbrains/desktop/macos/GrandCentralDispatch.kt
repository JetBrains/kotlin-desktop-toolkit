package org.jetbrains.desktop.macos

import org.jetbrains.desktop.IGrandCentralDispatch
import org.jetbrains.desktop.macos.generated.desktop_macos_h
import org.jetbrains.desktop.macos.generated.`dispatcher_main_exec_async$f`
import java.lang.foreign.Arena
import java.lang.foreign.MemorySegment
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.CountDownLatch

public object GrandCentralDispatch : IGrandCentralDispatch {
    private val queue = ConcurrentLinkedQueue<() -> Unit>()
    private val callback: MemorySegment = `dispatcher_main_exec_async$f`.allocate({
        ffiUpCall {
            queue.poll().invoke()
        }
    }, Arena.global())

    override fun isMainThread(): Boolean {
        return ffiDownCall {
            desktop_macos_h.dispatcher_is_main_thread()
        }
    }

    override fun dispatchOnMain(f: () -> Unit) {
        ffiDownCall {
            queue.add(f)
            desktop_macos_h.dispatcher_main_exec_async(callback)
        }
    }

    public fun <T> dispatchOnMainSync(f: () -> T): T {
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
