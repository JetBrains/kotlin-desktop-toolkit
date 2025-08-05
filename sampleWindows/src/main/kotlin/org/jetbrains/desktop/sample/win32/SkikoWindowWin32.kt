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
    private var renderTarget: BackendRenderTarget? = null
    private var surface: Surface? = null

    init {
        window.setMinSize(LogicalSize(320.0f, 240.0f))
    }

    private fun isSizeChanged(size: PhysicalSize): Boolean {
        if (size.width != currentSize.width || size.height != currentSize.height) {
            currentSize = size
            return true
        }
        return false
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowDraw -> {
                performDrawing(event.size, event.scale)
                EventHandlerResult.Stop
            }

            is Event.WindowScaleChanged -> {
                window.setRect(event.newOrigin, event.newSize)
                EventHandlerResult.Stop
            }

            is Event.WindowResize -> {
                window.requestUpdate()
                EventHandlerResult.Stop
            }

            else -> EventHandlerResult.Continue
        }
    }

    fun performDrawing(size: PhysicalSize, scale: Float) {
        if (isSizeChanged(size) || surface == null) {
            val surfaceParams = angleRenderer.makeSurface(currentSize.width, currentSize.height)
            renderTarget = BackendRenderTarget.makeGL(
                width = currentSize.width,
                height = currentSize.height,
                sampleCnt = 1,
                stencilBits = 8,
                fbId = surfaceParams.framebufferBinding,
                fbFormat = FramebufferFormat.GR_GL_RGBA8,
            )
            surface = Surface.makeFromBackendRenderTarget(
                context = directContext,
                rt = renderTarget!!,
                origin = SurfaceOrigin.BOTTOM_LEFT,
                colorFormat = SurfaceColorFormat.RGBA_8888,
                colorSpace = ColorSpace.sRGB,
                surfaceProps = null,
            )
        }
        surface!!.let { surface ->
            val time = creationTime.elapsedNow().inWholeMilliseconds
            angleRenderer.draw(true) {
                surface.canvas.draw(currentSize, scale, time)
                surface.flushAndSubmit()
            }
        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Float, time: Long)

    override fun close() {
        window.close()
    }
}
