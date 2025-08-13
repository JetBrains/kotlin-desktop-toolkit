package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.LogLevel
import org.junit.jupiter.api.AfterAll
import org.junit.jupiter.api.BeforeAll
import org.junit.jupiter.api.Timeout
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

/**
 * We expect that every test class will be executed with separate JVM instance, without parralle forks.
 * Tests from one test class also should be executed sequentially.
 * This is requirement because tests interacts with NSApplication which can be initialized only once per process,
 * moreover often the test might change OS state shared across different processes, so it's better to not run it
 * in parrallel event in separate processes.
 */
open class KDTTestBase {
    companion object {
        @BeforeAll
        @JvmStatic
        fun loadLibrary() {
            KotlinDesktopToolkit.init(consoleLogLevel = LogLevel.Error)
        }
    }
}

open class KDTApplicationTestBase : KDTTestBase() {

    fun <T> withEventHandler(handler: (Event) -> EventHandlerResult, body: () -> T): T {
        eventHandler = handler
        val result = body()
        eventHandler = null
        return result
    }

    fun <T> ui(body: () -> T): T = GrandCentralDispatch.dispatchOnMainSync(highPriority = false, body)

    companion object {
        var eventHandler: ((Event) -> EventHandlerResult)? = null

        @Volatile
        lateinit var handle: Thread

        @Timeout(value = 5, unit = TimeUnit.SECONDS)
        @BeforeAll
        @JvmStatic
        fun startApplication() {
            val applicationStartedLatch = CountDownLatch(1)
            handle = thread {
                GrandCentralDispatch.startOnMainThread {
                    Application.init()
                    Application.runEventLoop { event ->
                        if (event is Event.ApplicationDidFinishLaunching) {
                            applicationStartedLatch.countDown()
                        }
                        eventHandler?.invoke(event) ?: EventHandlerResult.Continue
                    }
                    GrandCentralDispatch.close()
                }
            }
            applicationStartedLatch.await()
        }

        @Timeout(value = 5, unit = TimeUnit.SECONDS)
        @AfterAll
        @JvmStatic
        fun stopApplication() {
            GrandCentralDispatch.dispatchOnMain {
                Application.stopEventLoop()
            }
            handle.join()
        }
    }
}

fun jbIconBytes(): ByteArray {
    return object {}.javaClass.getResource("/jb-logo.png")!!.readBytes()
}
