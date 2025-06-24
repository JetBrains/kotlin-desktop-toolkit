package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.WindowParams
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.FramebufferFormat
import org.jetbrains.skia.GLAssembledInterface
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import org.jetbrains.skia.makeGLWithInterface
import kotlin.time.TimeSource

abstract class SkikoWindowWin32(
    params: WindowParams,
) : AutoCloseable {
//    private val directContext: DirectContext by lazy {
//        val eglFunc = app.getEglProcFunc()!!
//        val openGlInterace = GLAssembledInterface.createFromNativePointers(ctxPtr = eglFunc.ctxPtr, fPtr = eglFunc.fPtr)
//        DirectContext.makeGLWithInterface(openGlInterace)
//    }
    val window = Application.createWindow(params)
    private val creationTime = TimeSource.Monotonic.markNow()

    init {
        //window.setMinSize(LogicalSize(320.0f, 240.0f))
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowDraw -> {
                performDrawing(event)
                EventHandlerResult.Stop
            }
            else -> EventHandlerResult.Continue
        }
    }

    fun performDrawing(event: Event.WindowDraw) {
//        BackendRenderTarget.makeGL(
//            width = event.size.width,
//            height = event.size.height,
//            sampleCnt = 1,
//            stencilBits = 8,
//            fbId = 0,
//            fbFormat = FramebufferFormat.GR_GL_RGBA8,
//        ).use { renderTarget ->
//            Surface.makeFromBackendRenderTarget(
//                context = directContext,
//                rt = renderTarget,
//                origin = SurfaceOrigin.BOTTOM_LEFT,
//                colorFormat = SurfaceColorFormat.RGBA_8888,
//                colorSpace = ColorSpace.sRGB,
//                surfaceProps = null,
//            )!!.use { surface ->
//                val time = creationTime.elapsedNow().inWholeMilliseconds
//                surface.canvas.draw(event.size, event.scale, time)
//                surface.flushAndSubmit()
//            }
//        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Float, time: Long)

    override fun close() {
        window.close()
    }
}
