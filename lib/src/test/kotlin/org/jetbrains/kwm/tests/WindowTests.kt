package org.jetbrains.kwm.tests

import org.jetbrains.kwm.macos.GrandCentralDispatch
import org.jetbrains.kwm.macos.Window
import org.junit.jupiter.api.Test

class WindowTests {
    @Test
    fun smokeTest() {
        val window1 = GrandCentralDispatch.dispatchOnMainSync {
            Window.create("Hello1", 100f, 200f)
        }
        val window2 = GrandCentralDispatch.dispatchOnMainSync {
            Window.create("Hello2", 200f, 300f)
        }
        GrandCentralDispatch.dispatchOnMainSync {
            window1.close()
            window2.close()
        }
    }
}