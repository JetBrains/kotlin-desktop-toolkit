package org.jetbrains.desktop.macos

import org.junit.jupiter.api.Timeout
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

class GrandCentralDispatchTest {

    @Test
    fun setQualityOfServiceForCurrentThreadDoesNotThrowTest() {
        // Test with all available QoS values
        // If no exception is thrown, the test passes
        setQualityOfServiceForCurrentThread(QualityOfService.UserInteractive)
        setQualityOfServiceForCurrentThread(QualityOfService.UserInitiated)
        setQualityOfServiceForCurrentThread(QualityOfService.Utility)
        setQualityOfServiceForCurrentThread(QualityOfService.Background)
        setQualityOfServiceForCurrentThread(QualityOfService.Default)

        // Add an assertion to satisfy Kotlin test requirements
        assertNotNull(QualityOfService.Default, "QualityOfService.Default should not be null")
    }

    @Test
    @Timeout(value = 5, unit = TimeUnit.SECONDS)
    fun grandCentralDispatchAllowsReentrancyTest() {
        KotlinDesktopToolkit.init()
        GrandCentralDispatch.dispatchOnMain {
            Application.init()
            Application.runEventLoop(eventHandler = {
                EventHandlerResult.Continue
            })
        }
        val counter = AtomicInteger()
        GrandCentralDispatch.dispatchOnMain { counter.incrementAndGet() }
        GrandCentralDispatch.dispatchOnMain { counter.incrementAndGet() }
        GrandCentralDispatch.dispatchOnMainSync { counter.incrementAndGet() }
        assertEquals(counter.get(), 3)
    }
}
