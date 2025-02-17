package org.jetbrains.desktop.tests

import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import java.util.concurrent.atomic.AtomicInteger
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class GrandCentralDispatchTest {
    private fun doTest(body: () -> Unit) {
        KotlinDesktopToolkit.init()
        body()
    }

    @Test
    fun `current thread is not main`() = doTest {
        assertFalse(GrandCentralDispatch.isMainThread())
    }

    @Test
    fun `after dispatch current thread is main`() = doTest {
        assertFalse(GrandCentralDispatch.isMainThread())
        GrandCentralDispatch.dispatchOnMain {
            assertTrue(GrandCentralDispatch.isMainThread())
        }
        assertFalse(GrandCentralDispatch.isMainThread())
    }

    @Test
    fun `all tasks are executed exactly once`() = doTest {
        val tasksCount = 10000
        val counter = AtomicInteger(tasksCount)
        for (i in 0 until tasksCount) {
            GrandCentralDispatch.dispatchOnMain {
                counter.decrementAndGet()
            }
        }
        GrandCentralDispatch.dispatchOnMainSync {}
        assertEquals(0, counter.get())
    }

    @Test
    fun `nested invocations`() = doTest {
        val actions = mutableListOf<Int>()
        GrandCentralDispatch.dispatchOnMain {
            actions.add(1)
            GrandCentralDispatch.dispatchOnMain {
                actions.add(3)
            }
            actions.add(2)
        }
        GrandCentralDispatch.dispatchOnMainSync {}
        assertEquals(listOf(1, 2, 3), actions)
    }

    @Test
    fun `lambdas are executed in dispatch order`() = doTest {
        val invocationOrder = mutableListOf<Int>()
        val executionOrder = mutableListOf<Int>()
        for (i in 0..100) {
            invocationOrder.add(i)
            GrandCentralDispatch.dispatchOnMain {
                executionOrder.add(i)
            }
        }
        GrandCentralDispatch.dispatchOnMainSync {}
        assertEquals(invocationOrder, executionOrder)
    }
}
