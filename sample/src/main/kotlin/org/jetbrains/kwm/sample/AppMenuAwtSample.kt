package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.*
import java.awt.Dimension
import java.awt.Point
import java.awt.Toolkit
import javax.swing.JFrame
import kotlin.concurrent.thread

fun main() {
    Logger.info { runtimeInfo() }
    /// Toolkit initialization will instansiate NSApplication
    val toolkit = Toolkit.getDefaultToolkit()
    GrandCentralDispatch.dispatchOnMainSync {
        AppMenuManager.setMainMenuToNone()
        AppMenuManager.setMainMenu(buildAppMenu())
    }
    JFrame().apply {
        title = "Window1"
        size = Dimension(800, 600)
        location = Point(200, 200)
        isVisible = true
    }
    JFrame().apply {
        title = "Window2"
        size = Dimension(800, 600)
        location = Point(300, 300)
        isVisible = true
    }
    GrandCentralDispatch.dispatchOnMainSync {
        AppMenuManager.setMainMenuToNone()
    }
    thread {
        while (true) {
            GrandCentralDispatch.dispatchOnMainSync {
                AppMenuManager.setMainMenuToNone()
                AppMenuManager.setMainMenu(buildAppMenu())
            }
            Thread.sleep(1000)
        }
    }
}