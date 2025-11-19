package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.PhysicalSize
import org.jetbrains.desktop.linux.SoftwareDrawData
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.ColorAlphaType
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.ColorType
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.FramebufferFormat
import org.jetbrains.skia.GLAssembledInterface
import org.jetbrains.skia.ImageInfo
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import org.jetbrains.skia.makeGLWithInterface
import kotlin.time.TimeSource

internal fun performSoftwareDrawing(size: PhysicalSize, softwareDrawData: SoftwareDrawData, draw: (Surface) -> Boolean): Boolean {
    return Surface.makeRasterDirect(
        imageInfo = ImageInfo(
            width = size.width,
            height = size.height,
            colorType = ColorType.BGRA_8888,
            alphaType = ColorAlphaType.OPAQUE,
            colorSpace = ColorSpace.sRGB,
        ),
        pixelsPtr = softwareDrawData.canvas,
        rowBytes = softwareDrawData.stride,
        surfaceProps = null,
    ).use(draw)
}

internal fun performOpenGlDrawing(size: PhysicalSize, context: DirectContext, draw: (Surface) -> Boolean): Boolean {
    return BackendRenderTarget.makeGL(
        width = size.width,
        height = size.height,
        sampleCnt = 1,
        stencilBits = 8,
        fbId = 0,
        fbFormat = FramebufferFormat.GR_GL_RGBA8,
    ).use { renderTarget ->
        Surface.makeFromBackendRenderTarget(
            context = context,
            rt = renderTarget,
            origin = SurfaceOrigin.BOTTOM_LEFT,
            colorFormat = SurfaceColorFormat.RGBA_8888,
            colorSpace = ColorSpace.sRGB,
            surfaceProps = null,
        )!!.use(draw)
    }
}

abstract class SkikoWindowLinux(
    app: Application,
    params: WindowParams,
) : AutoCloseable {
    private val directContext: DirectContext by lazy {
        val eglFunc = app.getEglProcFunc()!!
        val openGlInterface = GLAssembledInterface.createFromNativePointers(ctxPtr = eglFunc.ctxPtr, fPtr = eglFunc.fPtr)
        DirectContext.makeGLWithInterface(openGlInterface)
    }
    val window = app.createWindow(params)
    private val creationTime = TimeSource.Monotonic.markNow()

    fun performDrawing(event: Event.WindowDraw): Boolean {
        val draw = { surface: Surface ->
            val time = creationTime.elapsedNow().inWholeMilliseconds
            surface.canvas.draw(event.size, event.scale, time)
            surface.flushAndSubmit()
            true
        }
        return event.softwareDrawData?.let { softwareDrawData ->
            performSoftwareDrawing(event.size, softwareDrawData, draw)
        } ?: performOpenGlDrawing(event.size, directContext, draw)
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Double, time: Long)

    override fun close() {
        window.close()
    }
}
