package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.junit.jupiter.api.assertThrows
import java.util.concurrent.atomic.AtomicInteger
import kotlin.test.Test
import kotlin.test.assertEquals

class GrandCentralDispatchCloseTest : KDTTestBase() {
    @Test
    fun `check that all tasks are executed before GCD is closed`() {
        val tasksCount = 10000
        val counter = AtomicInteger(0)

        GrandCentralDispatch.startOnMainThread {
            repeat(tasksCount) {
                GrandCentralDispatch.dispatchOnMain {
                    counter.incrementAndGet()
                }
            }
            GrandCentralDispatch.close()
        }
        assertEquals(tasksCount, counter.get())
        assertThrows<Throwable> {
            GrandCentralDispatch.dispatchOnMain { }
        }
    }
}
