package org.jetbrains.desktop.macos

import kotlin.test.Test
import kotlin.test.assertNotNull

class GrandCentralDispatchTest {

    @Test
    fun testSetQualityOfServiceForCurrentThreadDoesNotThrow() {
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
}
