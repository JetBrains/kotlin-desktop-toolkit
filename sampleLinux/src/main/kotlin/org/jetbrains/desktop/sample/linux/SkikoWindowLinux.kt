package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.PhysicalSize
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.ColorAlphaType
import org.jetbrains.skia.ColorType
import org.jetbrains.skia.ImageInfo
import org.jetbrains.skia.Surface
import kotlin.time.TimeSource

abstract class SkikoWindowLinux(
    app: Application,
    params: WindowParams,
) : AutoCloseable {
//    private val directContext = DirectContext.makeGL()
    val window = app.createWindow({ event -> handleEvent(event) }, params)
    private val creationTime = TimeSource.Monotonic.markNow()
    private val backgroundColor = Color.makeRGB(128, 128, 128)

    init {
//        window.minSize = LogicalSize(320.0, 240.0)
//        view.isOpaque = false
//        window.attachView(view)
    }

    private fun logEvents(event: Event) {
        when (event) {
            is Event.KeyDown -> {
                Logger.info { "$event" }
            }
            is Event.KeyUp -> {
                Logger.info { "$event" }
            }
            is Event.ModifiersChanged -> {
                Logger.info { "$event" }
            }
            is Event.MouseEntered -> {
                Logger.info { "$event" }
            }
            is Event.MouseExited -> {
                Logger.info { "$event" }
            }
            is Event.MouseMoved -> {
                Logger.info { "$event" }
            }
            is Event.MouseDragged -> {
                Logger.info { "$event" }
            }
            is Event.MouseDown -> {
                Logger.info { "$event" }
            }
            is Event.MouseUp -> {
                Logger.info { "$event" }
            }
            is Event.WindowDraw -> {
                performDrawing(event)
            }
            else -> {}
        }
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        logEvents(event)
        return when (event) {
            is Event.WindowScreenChange -> {
                EventHandlerResult.Continue
            }
            else -> EventHandlerResult.Continue
        }
    }

    fun performDrawing(event: Event.WindowDraw) {
//        BackendRenderTarget.makeGL(
//            width = event.width,
//            height = event.height,
//            sampleCnt = 1,
//            stencilBits = 8,
//            fbId = 0,
//            fbFormat = FramebufferFormat.GR_GL_RGBA8,
//        ).use { renderTarget ->
        Surface.makeRasterDirect(
            imageInfo = ImageInfo(
                width = event.width,
                height = event.height,
                colorType = ColorType.BGRA_8888,
                alphaType = ColorAlphaType.OPAQUE,
                colorSpace = null,
            ),
            pixelsPtr = event.buffer,
            rowBytes = event.stride,
            surfaceProps = null,
        ).use { surface ->
//            Surface.makeFromBackendRenderTarget(
//                context = directContext,
//                origin = SurfaceOrigin.TOP_LEFT,
//                colorFormat = SurfaceColorFormat.BGRA_8888,
//                colorSpace = ColorSpace.sRGB,
//                surfaceProps = null,
//                rt = renderTarget,
//            )!!.use { surface ->
            val time = creationTime.elapsedNow().inWholeMilliseconds
            surface.canvas.clear(backgroundColor)
            surface.canvas.draw(PhysicalSize(surface.width.toDouble(), surface.height.toDouble()), time)
            surface.flushAndSubmit()
        }
//        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, time: Long)

    override fun close() {
        window.close()
    }
}
