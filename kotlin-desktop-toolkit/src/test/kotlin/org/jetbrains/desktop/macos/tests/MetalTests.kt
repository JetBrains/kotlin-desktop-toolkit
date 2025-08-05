package org.jetbrains.desktop.macos.tests

import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.MetalCommandQueue
import org.jetbrains.desktop.macos.MetalDevice
import org.jetbrains.desktop.macos.MetalView
import org.jetbrains.desktop.macos.Window
import kotlin.test.Ignore
import kotlin.test.Test

class MetalTests: KDTApplicationTestBase() {
    @Test
    fun smokeTest() {
        val (device, queue) = ui {
            val device = MetalDevice.create()
            val queue = MetalCommandQueue.create(device)
            Pair(device, queue)
        }
        val view = ui {
            MetalView.create(device, onDisplayLayer = {})
        }
        val window = ui {
            val window = Window.create(
                origin = LogicalPoint(100.0, 100.0),
                title = "Hello",
            )
            window.attachView(view)
            window
        }
        ui {
            device.close()
            view.close()
            window.close()
            queue.close()
        }
    }
}
