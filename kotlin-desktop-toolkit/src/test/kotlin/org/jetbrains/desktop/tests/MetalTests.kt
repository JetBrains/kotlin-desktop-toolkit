package org.jetbrains.desktop.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.MetalCommandQueue
import org.jetbrains.desktop.macos.MetalDevice
import org.jetbrains.desktop.macos.MetalView
import org.jetbrains.desktop.macos.Window
import org.junit.jupiter.api.Test

class MetalTests {
    @Test
    fun smokeTest() {
        GrandCentralDispatch.dispatchOnMainSync {
            Application.init()
        }
        val (device, queue) = GrandCentralDispatch.dispatchOnMainSync {
            val device = MetalDevice.create()
            val queue = MetalCommandQueue.create(device)
            Pair(device, queue)
        }
        val view = GrandCentralDispatch.dispatchOnMainSync {
            MetalView.create(device, onDisplayLayer = {})
        }
        val window = GrandCentralDispatch.dispatchOnMainSync {
            val window = Window.create(
                origin = LogicalPoint(100.0, 100.0),
                title = "Hello",
            )
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
