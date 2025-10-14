package org.jetbrains.desktop.sample.win32

import org.jetbrains.desktop.win32.AngleRenderer
import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.Event
import org.jetbrains.desktop.win32.EventHandlerResult
import org.jetbrains.desktop.win32.Logger
import org.jetbrains.desktop.win32.LogicalSize
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.Screen
import org.jetbrains.desktop.win32.SurfaceParams
import org.jetbrains.desktop.win32.VirtualKey
import org.jetbrains.desktop.win32.WindowParams
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
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
    app: Application,
    params: WindowParams,
) : AutoCloseable {
    private val angleRenderer: AngleRenderer by lazy {
        AngleRenderer.create(window)
    }

    private val directContext: DirectContext by lazy {
        val eglFunc = angleRenderer.getEglGetProcFunc()
        val glInterface = GLAssembledInterface.createFromNativePointers(ctxPtr = eglFunc.ctxPtr, fPtr = eglFunc.fPtr)
        DirectContext.makeGLWithInterface(glInterface)
    }

    val window = app.createWindow(params)
    private val creationTime = TimeSource.Monotonic.markNow()

    private var currentSize = PhysicalSize(0, 0)
    private var surfaceParams: SurfaceParams? = null

    init {
        window.setMinSize(LogicalSize(320.0f, 240.0f))
    }

    private fun isSizeChanged(size: PhysicalSize): Boolean {
        return (size.width != currentSize.width || size.height != currentSize.height)
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowDraw -> with(event) {
                performDrawing(size, scale)
                EventHandlerResult.Stop
            }

            is Event.NCCalcSize -> with(event) {
                performDrawing(size, scale)
                EventHandlerResult.Stop
            }

            is Event.KeyDown -> {
                if (event.keyCode == VirtualKey.S) {
                    val screens = Screen.allScreens()
                    for (screen in screens) {
                        Logger.debug { "Screen: $screen" }
                    }
                }
                EventHandlerResult.Continue
            }

            else -> EventHandlerResult.Continue
        }
    }

    private fun performDrawing(size: PhysicalSize, scale: Float) {
        angleRenderer.makeCurrent()
        val doResizeSurface = isSizeChanged(size)
        if (doResizeSurface) {
            currentSize = size
            surfaceParams = angleRenderer.resizeSurface(size.width, size.height)
        }
        BackendRenderTarget.makeGL(
            width = size.width,
            height = size.height,
            sampleCnt = 1,
            stencilBits = 8,
            fbId = surfaceParams!!.framebufferBinding,
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
                surface.canvas.clear(Color.TRANSPARENT)
                surface.canvas.draw(size, scale, time)
                surface.flushAndSubmit()
                angleRenderer.swapBuffers(waitForVsync = !doResizeSurface)
            }
        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Float, time: Long)

    override fun close() {
        window.close()
    }
}
