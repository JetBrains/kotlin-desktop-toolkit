package org.jetbrains.desktop.sample.compose.windows

import org.jetbrains.desktop.win32.AngleRenderer
import org.jetbrains.desktop.win32.Application
import org.jetbrains.desktop.win32.FontSmoothing
import org.jetbrains.desktop.win32.FontSmoothingOrientation
import org.jetbrains.desktop.win32.FontSmoothingType
import org.jetbrains.desktop.win32.PhysicalSize
import org.jetbrains.desktop.win32.SurfaceParams
import org.jetbrains.desktop.win32.Window
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.Canvas
import org.jetbrains.skia.Color
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.FramebufferFormat
import org.jetbrains.skia.GLAssembledInterface
import org.jetbrains.skia.PixelGeometry
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import org.jetbrains.skia.SurfaceProps
import org.jetbrains.skia.makeGLWithInterface

/**
 * The GPU surface of one window: an ANGLE (OpenGL ES over Direct3D) device attached to its composition
 * visual, and the Skia context drawing into it.
 *
 * Per window rather than per application, unlike the shared Metal device on macOS: the toolkit creates
 * the ANGLE device *for* a window, because the swapchain it presents through belongs to that window's
 * visual.
 *
 * Everything here runs on the event loop thread. There is no render thread and no frame queue —
 * [renderFrame] records and presents in one call, and [WindowsFramePump] is what keeps that from
 * happening more often than the display can show.
 */
internal class AngleViewContext private constructor(
    private val angleRenderer: AngleRenderer,
    private val directContext: DirectContext,
) : AutoCloseable {
    private var currentSize = PhysicalSize(0, 0)
    private var surfaceParams: SurfaceParams? = null

    /**
     * Draws one frame at [physicalSize] and presents it.
     *
     * The surface is resized lazily, on the first frame that needs a different size, so a window that is
     * being dragged by its edge reallocates once per size it actually renders at.
     */
    fun renderFrame(physicalSize: PhysicalSize, pixelGeometry: PixelGeometry, drawContent: Canvas.() -> Unit) {
        angleRenderer.makeCurrent()
        if (physicalSize != currentSize) {
            // Assigned only once `resizeSurface` has returned: if it throws, the next frame retries
            // against the same target size instead of drawing into a surface of the wrong size.
            surfaceParams = angleRenderer.resizeSurface(physicalSize.width, physicalSize.height)
            currentSize = physicalSize
        }
        val params = surfaceParams ?: return
        BackendRenderTarget.makeGL(
            width = physicalSize.width,
            height = physicalSize.height,
            sampleCnt = 1,
            stencilBits = 8,
            fbId = params.framebufferBinding,
            fbFormat = FramebufferFormat.GR_GL_RGBA8,
        ).use { renderTarget ->
            Surface.makeFromBackendRenderTarget(
                context = directContext,
                rt = renderTarget,
                origin = SurfaceOrigin.BOTTOM_LEFT,
                colorFormat = SurfaceColorFormat.RGBA_8888,
                colorSpace = ColorSpace.sRGB,
                surfaceProps = SurfaceProps(pixelGeometry = pixelGeometry),
            )!!.use { surface ->
                surface.canvas.clear(Color.TRANSPARENT)
                try {
                    surface.canvas.drawContent()
                } finally {
                    // Presented even when drawing threw, and only then is the exception allowed to
                    // propagate: an unvalidated update region makes Windows re-post `WM_PAINT` in a tight
                    // loop, which freezes the UI thread. A partially drawn frame beats a freeze.
                    surface.flushAndSubmit()
                    angleRenderer.swapBuffers()
                }
            }
        }
    }

    override fun close() {
        angleRenderer.makeCurrent()
        directContext.close()
        angleRenderer.close()
    }

    companion object {
        fun create(application: Application, nativeWindow: Window): AngleViewContext {
            val angleRenderer = application.createAngleRenderer(nativeWindow)
            val eglGetProcFunc = angleRenderer.getEglGetProcFunc()
            angleRenderer.makeCurrent()
            val glInterface = GLAssembledInterface.createFromNativePointers(
                ctxPtr = eglGetProcFunc.ctxPtr,
                fPtr = eglGetProcFunc.fPtr,
            )
            return AngleViewContext(angleRenderer, DirectContext.makeGLWithInterface(glInterface))
        }
    }
}

/**
 * How Skia should lay out subpixels when rasterising text, from the system's ClearType settings.
 *
 * Unknown — grayscale antialiasing — whenever ClearType is not what the user asked for; the RGB/BGR
 * order otherwise, because that is the physical order of the display's subpixels and getting it wrong
 * fringes every glyph.
 */
internal fun currentPixelGeometry(): PixelGeometry {
    if (FontSmoothing.getCurrent() == FontSmoothing.Disabled) {
        return PixelGeometry.UNKNOWN
    }
    if (FontSmoothingType.getCurrent() != FontSmoothingType.ClearType) {
        return PixelGeometry.UNKNOWN
    }
    return when (FontSmoothingOrientation.getCurrent()) {
        FontSmoothingOrientation.Rgb -> PixelGeometry.RGB_H
        FontSmoothingOrientation.Bgr -> PixelGeometry.BGR_H
    }
}
