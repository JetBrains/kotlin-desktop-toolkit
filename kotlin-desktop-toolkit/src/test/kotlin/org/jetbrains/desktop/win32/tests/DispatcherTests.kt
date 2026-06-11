package org.jetbrains.desktop.win32.tests

import org.jetbrains.annotations.TestOnly
import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.KotlinDesktopToolkit
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.nio.file.Path
import java.util.concurrent.Callable
import java.util.concurrent.Executors
import kotlin.concurrent.atomics.AtomicInt
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.concurrent.atomics.fetchAndDecrement
import kotlin.concurrent.atomics.fetchAndIncrement
import kotlin.concurrent.thread
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.use

@EnabledOnOs(OS.WINDOWS)
@OptIn(ExperimentalAtomicApi::class)
class DispatcherTests {
    companion object {
        @BeforeAll
        @JvmStatic
        fun loadLibrary() {
            KotlinDesktopToolkit.init(
                libraryFolderPath = Path.of(System.getProperty("kdt.win32.library.folder.path")!!),
            )
        }
    }

    @Test
    fun `app thread is main`() {
        Application.runOnce { app ->
            assertTrue { app.isDispatcherThread() }
        }
    }

    @Test
    fun `background thread is not main`() {
        Application.runOnce { app ->
            thread(name = "background thread") {
                assertFalse { app.isDispatcherThread() }
            }
        }
    }

    @Test
    fun `background threads dispatch to main`() {
        val threadsCount = 10
        val counter = AtomicInt(0)
        Application.runOnce { app ->
            val mainThreadId = Thread.currentThread().threadId()
            val threads = mutableListOf<Callable<Unit>>()
            val executor = Executors.newFixedThreadPool(threadsCount)
            (0 until threadsCount).forEach { _ ->
                threads.add {
                    assertNotEquals(mainThreadId, Thread.currentThread().threadId())
                    app.invokeOnDispatcher {
                        assertEquals(mainThreadId, Thread.currentThread().threadId())
                        counter.fetchAndIncrement()
                    }
                }
            }
            executor.invokeAll(threads)
        }
        assertEquals(threadsCount, counter.load())
    }

    @Test
    fun `current thread after dispatch is not main`() {
        var innerCallbackRan = false
        Application.runOnce { app ->
            assertTrue { app.isDispatcherThread() }
            thread(name = "background thread") {
                assertFalse { app.isDispatcherThread() }
                app.invokeOnDispatcher {
                    assertTrue { app.isDispatcherThread() }
                    innerCallbackRan = true
                }
                assertFalse { app.isDispatcherThread() }
            }.join()
            assertTrue { app.isDispatcherThread() }
        }
        assertTrue(innerCallbackRan, "Inner callback should be run")
    }

    @Test
    fun `all tasks are executed exactly once`() {
        val tasksCount = 10000
        val counter = AtomicInt(tasksCount)
        Application.runOnce { app ->
            (0 until tasksCount).forEach { _ ->
                app.invokeOnDispatcher {
                    counter.fetchAndDecrement()
                }
            }
        }
        assertEquals(0, counter.load())
    }

    @Test
    fun `tasks are executed in dispatch order`() {
        val invocationOrder = mutableListOf<Int>()
        val executionOrder = mutableListOf<Int>()
        Application.runOnce { app ->
            for (i in 0..100) {
                invocationOrder.add(i)
                app.invokeOnDispatcher {
                    executionOrder.add(i)
                }
            }
        }
        assertEquals(invocationOrder, executionOrder)
    }
}

@TestOnly
private fun Application.Companion.runOnce(action: (Application) -> Unit) {
    Application().use { app ->
        app.onStartup {
            action(app)
            app.invokeOnDispatcher { app.stopEventLoop() }
        }
        app.runEventLoop { _, _ -> EventHandlerResult.Continue }
    }
}
