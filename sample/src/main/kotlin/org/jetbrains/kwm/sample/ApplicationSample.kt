package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.*
import kotlin.concurrent.thread


fun main() {
    printRuntimeInfo()
    Application.initWithConfig(Application.Config(
//        disableDictationMenuItem = true,
//        disableCharacterPaletteMenuItem = true
    ))
    AppMenuManager.setMainMenu(buildAppMenu())
    Application.createWindow("Window1", 100f, 200f)
    Application.createWindow("Window2", 200f, 300f)
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