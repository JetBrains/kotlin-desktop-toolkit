package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.LogicalSize
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

abstract class SkikoWindowLinux(
    app: Application,
    params: WindowParams,
) : AutoCloseable {
    private val directContext: DirectContext by lazy {
        val eglFunc = app.getEglProcFunc()!!
        val openGlInterace = GLAssembledInterface.createFromNativePointers(ctxPtr = eglFunc.ctxPtr, fPtr = eglFunc.fPtr)
        DirectContext.makeGLWithInterface(openGlInterace)
    }
    val window = app.createWindow(params)
    private val creationTime = TimeSource.Monotonic.markNow()

    init {
        window.setMinSize(LogicalSize(320.0f, 240.0f))
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowDraw -> {
                performDrawing(event)
                EventHandlerResult.Stop
            }
            is Event.WindowScreenChange -> {
                EventHandlerResult.Continue
            }
            else -> EventHandlerResult.Continue
        }
    }

    fun performSoftwareDrawing(event: Event.WindowDraw, softwareDrawData: SoftwareDrawData) {
        Surface.makeRasterDirect(
            imageInfo = ImageInfo(
                width = event.size.width,
                height = event.size.height,
                colorType = ColorType.BGRA_8888,
                alphaType = ColorAlphaType.OPAQUE,
                colorSpace = ColorSpace.sRGB,
            ),
            pixelsPtr = softwareDrawData.canvas,
            rowBytes = softwareDrawData.stride,
            surfaceProps = null,
        ).use { surface ->
            val time = creationTime.elapsedNow().inWholeMilliseconds
            surface.canvas.draw(PhysicalSize(surface.width, surface.height), event.scale, time)
            surface.flushAndSubmit()
        }
    }

    fun performDrawing(event: Event.WindowDraw) {
        val softwareDrawData = event.softwareDrawData
        if (softwareDrawData != null) {
            performSoftwareDrawing(event, softwareDrawData)
            return
        }

        BackendRenderTarget.makeGL(
            width = event.size.width,
            height = event.size.height,
            sampleCnt = 1,
            stencilBits = 8,
            fbId = 0,
            fbFormat = FramebufferFormat.GR_GL_RGBA8,
        ).use { renderTarget ->
            Surface.makeFromBackendRenderTarget(
                context = directContext,
                rt = renderTarget,
                origin = SurfaceOrigin.BOTTOM_LEFT,
                colorFormat = SurfaceColorFormat.RGBA_8888,
                colorSpace = ColorSpace.sRGB,
                surfaceProps = null,
            )!!.use { surface ->
                val time = creationTime.elapsedNow().inWholeMilliseconds
                surface.canvas.draw(event.size, event.scale, time)
                surface.flushAndSubmit()
            }
        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Double, time: Long)

    override fun close() {
        window.close()
    }
}
