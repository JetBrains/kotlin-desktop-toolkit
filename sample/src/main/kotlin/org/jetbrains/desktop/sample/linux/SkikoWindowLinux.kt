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
import kotlin.concurrent.atomics.AtomicBoolean
import kotlin.concurrent.atomics.AtomicLong
import kotlin.concurrent.atomics.ExperimentalAtomicApi
import kotlin.math.floor
import kotlin.math.roundToLong
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

@OptIn(ExperimentalAtomicApi::class)
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
    private val drawRequesterActive = AtomicBoolean(false)
    private val frameSleepDurationMs = AtomicLong(16)
    private var drawRequester = Thread({
        while (drawRequesterActive.load()) {
            app.runOnEventLoopAsync {
                if (drawRequesterActive.load()) {
                    window.requestRedraw()
                }
            }
            Thread.sleep(frameSleepDurationMs.load())
        }
    })

    fun setMillihertz(millihertz: UInt) {
        val frameTime = 1_000_000.0 / millihertz.toDouble()
        frameSleepDurationMs.store(floor(frameTime).roundToLong())
    }

    fun performDrawing(event: Event.WindowDraw): Boolean {
        val draw = { surface: Surface ->
            val time = creationTime.elapsedNow().inWholeMilliseconds
            surface.canvas.draw(event.size, event.scale, time)
            surface.flushAndSubmit()
            true
        }
        if (drawRequesterActive.compareAndSet(expectedValue = false, newValue = true)) {
            drawRequester.start()
        }
        return event.softwareDrawData?.let { softwareDrawData ->
            performSoftwareDrawing(event.size, softwareDrawData, draw)
        } ?: run {
            performOpenGlDrawing(event.size, directContext, draw)
        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, scale: Double, time: Long)

    override fun close() {
        drawRequesterActive.store(false)
        window.close()
    }
}
