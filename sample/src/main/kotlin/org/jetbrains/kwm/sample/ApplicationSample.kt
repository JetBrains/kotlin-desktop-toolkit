package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.AppMenuManager
import org.jetbrains.kwm.macos.Application
import org.jetbrains.kwm.macos.GrandCentralDispatch
import kotlin.concurrent.thread


fun main() {
    printRuntimeInfo()
    Application.init()
    AppMenuManager.setMainMenu(buildAppMenu())
    thread {
        while (true) {
            GrandCentralDispatch.dispatchOnMain {
                AppMenuManager.setMainMenu(buildAppMenu())
            }
            Thread.sleep(1000)
        }
    }
    Application.runEventLoop()
}