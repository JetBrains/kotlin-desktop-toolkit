package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.AngleRenderer
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

abstract class SkikoWindowWin32(params: WindowParams) : AutoCloseable {
    private val angleRenderer: AngleRenderer by lazy {
        AngleRenderer.create(window)
    }

    private val directContext: DirectContext by lazy {
        val eglFunc = angleRenderer.getEglGetProcFunc()
        val glInterface = GLAssembledInterface.createFromNativePointers(ctxPtr = eglFunc.ctxPtr, fPtr = eglFunc.fPtr)
        DirectContext.makeGLWithInterface(glInterface)
    }

    val window = Application.createWindow(params)
    private val creationTime = TimeSource.Monotonic.markNow()

    private var currentSize = PhysicalSize(0, 0)
    private var surface: Surface? = null

    init {
        window.setMinSize(LogicalSize(320.0f, 240.0f))
    }

    private fun isSizeChanged(size: PhysicalSize): Boolean {
        return (size.width != currentSize.width || size.height != currentSize.height)
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowDraw -> {
                performDrawing(event.size, event.scale)
                EventHandlerResult.Stop
            }

            is Event.WindowScaleChanged -> {
                window.setRect(event.origin, event.size)
                EventHandlerResult.Stop
            }

            is Event.WindowPositionChanging -> {
                if (isSizeChanged(event.size)) {
                    makeSurface(event.size, event.scale)
                    angleRenderer.swapBuffers(true)
                }
                EventHandlerResult.Stop
            }

            else -> EventHandlerResult.Continue
        }
    }

    fun performDrawing(size: PhysicalSize, scale: Float) {
        surface!!.let { surface ->
            val time = creationTime.elapsedNow().inWholeMilliseconds
            angleRenderer.draw(true) {
                surface.canvas.draw(size, scale, time)
                surface.flushAndSubmit()
            }
        }
    }

    fun makeSurface(size: PhysicalSize, scale: Float) {
        currentSize = size
        val surfaceParams = angleRenderer.resizeSurface(size.width, size.height)
        surface = BackendRenderTarget.makeGL(
            width = size.width,
            height = size.height,
            sampleCnt = 1,
            stencilBits = 8,
            fbId = surfaceParams.framebufferBinding,
            fbFormat = FramebufferFormat.GR_GL_RGBA8,
        ).use { renderTarget ->
            Surface.makeFromBackendRenderTarget(
                context = directContext,
                rt = renderTarget,
                origin = SurfaceOrigin.BOTTOM_LEFT,
                colorFormat = SurfaceColorFormat.RGBA_8888,
                colorSpace = ColorSpace.sRGB,
                surfaceProps = null,
            )
        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Float, time: Long)

    override fun close() {
        window.close()
    }
}
