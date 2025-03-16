package org.jetbrains.desktop.sample

import org.jetbrains.desktop.macos.LogicalPoint
import org.jetbrains.desktop.macos.AppMenuManager
import org.jetbrains.desktop.macos.Application
import org.jetbrains.desktop.macos.EventHandlerResult
import org.jetbrains.desktop.macos.GrandCentralDispatch
import org.jetbrains.desktop.macos.KotlinDesktopToolkit
import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.Window
import kotlin.concurrent.thread

fun main() {
    KotlinDesktopToolkit.init()
    Logger.info { runtimeInfo() }
    Application.init(
        Application.ApplicationConfig(
//        disableDictationMenuItem = true,
//        disableCharacterPaletteMenuItem = true
        ),
    )
    AppMenuManager.setMainMenu(buildAppMenu())
    val window1 = Window.create(origin = LogicalPoint(100.0, 200.0), title = "Window1")
    val window2 = Window.create(origin = LogicalPoint(200.0, 300.0), title = "Window2")

    thread {
        while (true) {
            GrandCentralDispatch.dispatchOnMain {
                AppMenuManager.setMainMenu(buildAppMenu())
            }
            Thread.sleep(1000)
        }
    }
    Application.runEventLoop { EventHandlerResult.Continue }
    window1.close()
    window2.close()
}
