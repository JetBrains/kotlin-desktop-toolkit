package org.jetbrains.desktop.sample.linux

import org.jetbrains.desktop.linux.Application
import org.jetbrains.desktop.linux.Event
import org.jetbrains.desktop.linux.EventHandlerResult
import org.jetbrains.desktop.linux.Logger
import org.jetbrains.desktop.linux.PhysicalSize
import org.jetbrains.desktop.linux.WindowButtonType
import org.jetbrains.desktop.linux.WindowParams
import org.jetbrains.desktop.linux.XdgDesktopSetting
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.ColorAlphaType
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.ColorType
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.FramebufferFormat
import org.jetbrains.skia.ImageInfo
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import kotlin.time.TimeSource

data class XdgDesktopSettings(
    var titlebarLayout: XdgDesktopSetting.TitlebarLayout = XdgDesktopSetting.TitlebarLayout(
        layoutLeft = listOf(WindowButtonType.Icon),
        layoutRight = listOf(WindowButtonType.Minimize, WindowButtonType.Maximize, WindowButtonType.Close),
    ),
    var doubleClickIntervalMs: Int = 500,
) {
    fun update(s: XdgDesktopSetting) {
        when (s) {
            is XdgDesktopSetting.TitlebarLayout -> titlebarLayout = s
            is XdgDesktopSetting.DoubleClickInterval -> doubleClickIntervalMs = s.intervalMs
        }
    }
}

abstract class SkikoWindowLinux(
    app: Application,
    params: WindowParams,
) : AutoCloseable {
    private val directContext: DirectContext by lazy {
        val eglFunc = app.getEglProcFunc()!!
        val openGlInterace = DirectContext.makeGlAssembledInterface(ctxPtr = eglFunc.ctxPtr, fPtr = eglFunc.fPtr)
        DirectContext.makeGLWithInterface(openGlInterace)
    }
    val window = app.createWindow({ event -> handleEvent(event) }, params)
    private val creationTime = TimeSource.Monotonic.markNow()
    private val backgroundColor = Color.makeARGB(128, 128, 128, 128)

//    init {
//        window.minSize = LogicalSize(320.0, 240.0)
//        view.isOpaque = false
//        window.attachView(view)
//    }

    private fun logEvents(event: Event) {
        when (event) {
            is Event.KeyDown,
            is Event.KeyUp,
            is Event.ModifiersChanged,
            is Event.MouseEntered,
            is Event.MouseExited,
            is Event.MouseDragged,
            is Event.MouseDown,
            is Event.MouseUp,
            is Event.ScrollWheel,
            is Event.WindowCloseRequest,
            is Event.WindowFocusChange,
            is Event.WindowFullScreenToggle,
            is Event.WindowMove,
            is Event.WindowResize,
            is Event.WindowScaleChanged,
            is Event.WindowScreenChange,
            -> {
                Logger.info { "$event" }
            }
            is Event.MouseMoved, is Event.WindowDraw -> {
            }
        }
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        logEvents(event)
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

    fun performSoftwareDrawing(event: Event.WindowDraw) {
        Surface.makeRasterDirect(
            imageInfo = ImageInfo(
                width = event.size.width,
                height = event.size.height,
                colorType = ColorType.BGRA_8888,
                alphaType = ColorAlphaType.OPAQUE,
                colorSpace = ColorSpace.sRGB,
            ),
            pixelsPtr = event.buffer!!,
            rowBytes = event.stride,
            surfaceProps = null,
        ).use { surface ->
            val time = creationTime.elapsedNow().inWholeMilliseconds
            surface.canvas.clear(backgroundColor)
            surface.canvas.draw(PhysicalSize(surface.width, surface.height), time)
            surface.flushAndSubmit()
        }
    }

    fun performDrawing(event: Event.WindowDraw) {
        if (event.buffer != null) {
            performSoftwareDrawing(event)
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
                surface.canvas.clear(backgroundColor)
                surface.canvas.draw(event.size, time)
                surface.flushAndSubmit()
            }
        }
    }

    abstract fun Canvas.draw(size: PhysicalSize, time: Long)

    override fun close() {
        window.close()
    }
}
