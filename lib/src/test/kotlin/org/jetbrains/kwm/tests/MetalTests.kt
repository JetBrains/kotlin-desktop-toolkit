package org.jetbrains.kwm.tests

import org.jetbrains.kwm.LogicalPoint
import org.jetbrains.kwm.macos.*
import org.junit.jupiter.api.Test

class MetalTests {
    @Test
    fun smokeTest() {
        val (device, queue) = GrandCentralDispatch.dispatchOnMainSync {
            val device = MetalDevice.create()
            val queue = MetalCommandQueue.create(device)
            Pair(device, queue)
        }
        val view = GrandCentralDispatch.dispatchOnMainSync {
            MetalView.create(device)
        }
        val window = GrandCentralDispatch.dispatchOnMainSync {
            val window = Window.create(origin = LogicalPoint(100.0, 100.0),
                                       title = "Hello")
            window.attachView(view)
            window
        }
        GrandCentralDispatch.dispatchOnMainSync {
            device.close()
            view.close()
            window.close()
            queue.close()
        }
    }
}