package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.Event
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.junit.jupiter.api.Timeout
import org.junit.jupiter.api.condition.EnabledOnOs
import org.junit.jupiter.api.condition.OS
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread
import kotlin.test.Test

@EnabledOnOs(OS.MAC)
class ApplicationEventLoopTest : KDTTestBase() {

    fun initApplication() {
        GrandCentralDispatch.startOnMainThread {
            Application.init()
        }
    }

    fun startAndStopEventLoop() {
        val applicationStartedLatch = CountDownLatch(1)
        val handler = thread {
            GrandCentralDispatch.startOnMainThread {
                Application.runEventLoop { event ->
                    if (event is Event.ApplicationDidFinishLaunching) {
                        applicationStartedLatch.countDown()
                    }
                    EventHandlerResult.Continue
                }
            }
        }
        applicationStartedLatch.await()
        GrandCentralDispatch.dispatchOnMainSync {
            Application.stopEventLoop()
        }
        handler.join()
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun `we can start and stop event loop`() {
        initApplication()
        startAndStopEventLoop()
        // If you run event loop second time you don't get `ApplicationDidFinishLaunching`
        // event anymore
        // startAndStopEventLoop()
    }
}
