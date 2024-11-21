package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.*
import java.awt.Dimension
import java.awt.Point
import java.awt.Toolkit
import javax.swing.JFrame
import kotlin.concurrent.thread

fun main() {
    printRuntimeInfo()
    /// Toolkit initialization will instansiate NSApplication
    val toolkit = Toolkit.getDefaultToolkit()
    thread {
        while (true) {
            GrandCentralDispatch.dispatchOnMain {
                AppMenuManager.setMainMenu(buildAppMenu())
            }
            Thread.sleep(1000)
        }
    }
    JFrame().apply {
        size = Dimension(800, 600)
        location = Point(200, 200)
        isVisible = true
    }
}