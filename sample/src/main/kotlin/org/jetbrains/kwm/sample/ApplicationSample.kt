package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.*
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Surface
import org.jetbrains.skia.DirectContext
import kotlin.concurrent.thread


fun main() {
    printRuntimeInfo()
    Application.init(Application.Config(
//        disableDictationMenuItem = true,
//        disableCharacterPaletteMenuItem = true
    ))
    AppMenuManager.setMainMenu(buildAppMenu())
    val window1 = Window.create("Window1", 100f, 200f)
    val window2 = Window.create("Window2", 200f, 300f)

    thread {
        while (true) {
            GrandCentralDispatch.dispatchOnMain {
                AppMenuManager.setMainMenu(buildAppMenu())
            }
            Thread.sleep(1000)
        }
    }
    Application.runEventLoop() {
        window1.close()
        window2.close()
    }
}