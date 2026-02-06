package org.jetbrains.desktop.sample.gtk

import org.jetbrains.desktop.gtk.Application
import org.jetbrains.desktop.gtk.Event
import org.jetbrains.desktop.gtk.PhysicalSize
import org.jetbrains.desktop.gtk.WindowParams
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

internal fun performOpenGlDrawing(framebuffer: Int, size: PhysicalSize, context: DirectContext, draw: (Surface) -> Boolean): Boolean {
    return BackendRenderTarget.makeGL(
        width = size.width,
        height = size.height,
        sampleCnt = 1,
        stencilBits = 0,
        fbId = framebuffer,
        fbFormat = FramebufferFormat.GR_GL_RGBA8,
    ).use { renderTarget ->
        Surface.makeFromBackendRenderTarget(
            context = context,
            rt = renderTarget,
            origin = SurfaceOrigin.TOP_LEFT,
            colorFormat = SurfaceColorFormat.RGBA_8888,
            colorSpace = ColorSpace.sRGB,
            surfaceProps = null,
        )!!.use(draw)
    }
}

abstract class SkikoWindowGtk(
    app: Application,
    params: WindowParams,
    glProcFunc: Application.GlProcFunc,
) : AutoCloseable {
    private val directContext: DirectContext by lazy {
        val openGlInterface = GLAssembledInterface.createFromNativePointers(ctxPtr = glProcFunc.ctxPtr, fPtr = glProcFunc.fPtr)
        DirectContext.makeGLWithInterface(openGlInterface)
    }
    val window = app.createWindow(params)
    private val creationTime = TimeSource.Monotonic.markNow()

    fun performDrawing(event: Event.WindowDraw, scale: Double): Boolean {
        val draw = { surface: Surface ->
            val time = creationTime.elapsedNow().inWholeMilliseconds
            surface.canvas.draw(event.size, scale, time)
            surface.flushAndSubmit()
            true
        }
        return performOpenGlDrawing(framebuffer = event.openGlDrawData.framebuffer, event.size, directContext, draw)
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Double, time: Long)

    override fun close() {
        window.close()
    }
}
