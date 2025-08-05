package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.QualityOfService
import org.jetbrains.desktop.macos.setQualityOfServiceForCurrentThread
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

@EnabledOnOs(OS.MAC)
class GrandCentralDispatchTest : KDTTestBase() {

    @Test
    fun `current thread is not main`() {
        thread(name = "Background thread") {
            assertFalse(GrandCentralDispatch.isMainThread())
        }.join()
    }

    @Test
    fun `after dispatch current thread is main`() {
        thread(name = "Background thread") {
            assertFalse(GrandCentralDispatch.isMainThread())
            GrandCentralDispatch.dispatchOnMainSync {
                assertTrue(GrandCentralDispatch.isMainThread())
            }
            assertFalse(GrandCentralDispatch.isMainThread())
        }.join()
    }

    @Test
    fun `all tasks are executed exactly once`() {
        val tasksCount = 10000
        val counter = AtomicInteger(tasksCount)
        (0 until tasksCount).forEach { _ ->
            GrandCentralDispatch.dispatchOnMain {
                counter.decrementAndGet()
            }
        }
        GrandCentralDispatch.dispatchOnMainSync {}
        assertEquals(0, counter.get())
    }

    @Test
    fun `nested invocations`() {
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
    fun `lambdas are executed in dispatch order`() {
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

    @Test
    fun `hight priority tasks are executed first`() {
        val tasksCount = 100
        val executionOrder = mutableListOf<Int>()
        GrandCentralDispatch.dispatchOnMainSync {
            repeat(tasksCount) {
                GrandCentralDispatch.dispatchOnMain(highPriority = false) {
                    executionOrder.add(2)
                }
                GrandCentralDispatch.dispatchOnMain(highPriority = true) {
                    executionOrder.add(1)
                }
            }
        }
        val expectedOrder = buildList {
            repeat(tasksCount) { add(1) }
            repeat(tasksCount) { add(2) }
        }
        GrandCentralDispatch.dispatchOnMainSync {}
        assertEquals(expectedOrder, executionOrder)
    }

    @Test
    fun `dispatch sync rethrows exception`() {
        assertThrows<Error> {
            GrandCentralDispatch.dispatchOnMainSync {
                throw Error("Test exception")
            }
        }
    }

    @Test
    fun `set quality of service doesn't throw`() {
        setQualityOfServiceForCurrentThread(QualityOfService.UserInteractive)
        setQualityOfServiceForCurrentThread(QualityOfService.UserInitiated)
        setQualityOfServiceForCurrentThread(QualityOfService.Utility)
        setQualityOfServiceForCurrentThread(QualityOfService.Background)
        setQualityOfServiceForCurrentThread(QualityOfService.Default)
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun grandCentralDispatchAllowsReentrancyTest() {
        runTestWithEventLoop(eventHandler = { EventHandlerResult.Continue }) {
            val counter = AtomicInteger()
            GrandCentralDispatch.dispatchOnMain { counter.incrementAndGet() }
            GrandCentralDispatch.dispatchOnMain { counter.incrementAndGet() }
            GrandCentralDispatch.dispatchOnMainSync { counter.incrementAndGet() }
            assertEquals(counter.get(), 3)
        }
    }

    fun runTestWithEventLoop(eventHandler: (Event) -> EventHandlerResult, body: () -> Unit) {
        val applicationStartedLatch = CountDownLatch(1)
        val handler = thread {
            GrandCentralDispatch.startOnMainThread {
                Application.init()
                Application.runEventLoop { event ->
                    if (event is Event.ApplicationDidFinishLaunching) {
                        applicationStartedLatch.countDown()
                    }
                    eventHandler(event)
                }
            }
        }
        try {
            applicationStartedLatch.await()
            body()
        } finally {
            GrandCentralDispatch.dispatchOnMainSync {
                Application.stopEventLoop()
            }
            handler.join()
        }
    }
}
