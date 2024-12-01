package org.jetbrains.kwm.sample

import org.jetbrains.kwm.macos.*
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Surface
import org.jetbrains.skia.DirectContext
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

    // First option: [SkSurface.WrapBackendRenderTarget]
//    val renderTarget = BackendRenderTarget.makeMetal(width, height, texturePtr)
//    val context = DirectContext.makeMetal(devicePtr, queuePtr)
//    val surface = Surface.makeFromBackendRenderTarget(context, renderTarget, ...)
//    val canvas = surface.canvas

    // Second option
//    val context = DirectContext.makeMetal(devicePtr, queuePtr)
//    val mtkViewPtr = ...
//    Surface.makeFromMTKView(context, mtkViewPtr, ...)

    // Third option [Not available in Skiko yet]
//    Surface.WrapCAMetalLayer()


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