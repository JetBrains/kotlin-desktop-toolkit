package org.jetbrains.desktop.sample.compose

import org.jetbrains.desktop.macos.Logger
import org.jetbrains.desktop.macos.MetalCommandQueue
import org.jetbrains.desktop.macos.MetalDevice
import org.jetbrains.desktop.macos.MetalView
import org.jetbrains.desktop.macos.PhysicalSize
import org.jetbrains.desktop.macos.QualityOfService
import org.jetbrains.desktop.macos.setQualityOfServiceForCurrentThread
import org.jetbrains.desktop.macos.withAutoReleasePool
import org.jetbrains.skia.BackendRenderTarget
import org.jetbrains.skia.ColorSpace
import org.jetbrains.skia.DirectContext
import org.jetbrains.skia.Picture
import org.jetbrains.skia.Surface
import org.jetbrains.skia.SurfaceColorFormat
import org.jetbrains.skia.SurfaceOrigin
import java.util.concurrent.locks.ReentrantLock
import kotlin.concurrent.thread
import kotlin.concurrent.withLock

/**
 * A recorded frame together with the drawable size it was recorded for.
 *
 * The size is what lets the render thread detect that the window was resized after recording, in which
 * case the frame is stale and gets dropped in favour of the synchronous repaint the resize triggers.
 */
class PresentablePicture(
    val picture: Picture,
    val size: PhysicalSize,
) : AutoCloseable {
    override fun close() {
        picture.close()
    }
}

/** Indirection so the `onDisplayLayer` callback can be swapped after the [MetalView] is created. */
class CallbackReference(var callback: () -> Unit)

/**
 * Everything needed to turn a recorded [Picture] into pixels in one window.
 *
 * Compose records a frame on the main thread (cheap, no GPU work) and the actual Skia/Metal submission
 * happens on a dedicated render thread, so the main thread stays free to keep handling events.
 */
class MetalViewContext(
    val view: MetalView,
    private val onDisplayLayerCallbackReference: CallbackReference,
    val directContext: DirectContext,
    private val commandQueue: MetalCommandQueue,
) {
    var onDisplayLayer: () -> Unit
        get() = onDisplayLayerCallbackReference.callback
        set(value) {
            onDisplayLayerCallbackReference.callback = value
        }

    private class RenderTask(
        val picture: PresentablePicture,
        val waitForCATransaction: Boolean,
        val onComplete: () -> Unit,
    )

    private val renderTaskLock = ReentrantLock()
    private val hasPendingTask = renderTaskLock.newCondition()
    private var pendingTask: RenderTask? = null

    private val renderThread = thread(start = true, isDaemon = true, name = "Compose Render Thread") {
        setQualityOfServiceForCurrentThread(QualityOfService.UserInteractive)
        while (!Thread.currentThread().isInterrupted) {
            var task: RenderTask? = null
            try {
                task = renderTaskLock.withLock {
                    while (pendingTask == null) {
                        hasPendingTask.await()
                    }
                    pendingTask!!
                }
                presentSync(task.picture, task.waitForCATransaction)
            } catch (_: InterruptedException) {
                break
            } catch (e: Throwable) {
                Logger.error { "Error during rendering: $e" }
            } finally {
                renderTaskLock.withLock {
                    pendingTask = null
                }
                task?.onComplete?.invoke()
            }
        }
    }

    fun stopRenderThread() {
        renderThread.interrupt()
        renderThread.join()
    }

    fun presentAsync(picture: PresentablePicture, waitForCATransaction: Boolean, onComplete: () -> Unit = {}) {
        renderTaskLock.withLock {
            check(pendingTask == null) { "The previous frame isn't presented yet" }
            pendingTask = RenderTask(picture, waitForCATransaction, onComplete)
            hasPendingTask.signalAll()
        }
    }

    private val drawingLock = Any()

    fun presentSync(picture: PresentablePicture, waitForCATransaction: Boolean) {
        synchronized(drawingLock) {
            val size = view.size()
            if (size != picture.size) {
                // The window was resized after this frame was recorded; a synchronous repaint is coming.
                return
            }
            withAutoReleasePool {
                view.nextTexture().use { texture ->
                    BackendRenderTarget.makeMetal(size.width.toInt(), size.height.toInt(), texture.pointerAddress).use { renderTarget ->
                        Surface.makeFromBackendRenderTarget(
                            context = directContext,
                            origin = SurfaceOrigin.TOP_LEFT,
                            colorFormat = SurfaceColorFormat.BGRA_8888,
                            colorSpace = ColorSpace.sRGB,
                            surfaceProps = null,
                            rt = renderTarget,
                        )!!.use { surface ->
                            surface.canvas.drawPicture(picture.picture)
                            surface.flushAndSubmit()
                        }
                    }
                    view.present(commandQueue, waitForCATransaction = waitForCATransaction)
                }
            }
        }
    }
}

/**
 * The Metal device and command queue shared by every window of the application.
 */
class GpuContext : AutoCloseable {
    private val metalDevice = MetalDevice.create()
    private val metalCommandQueue = MetalCommandQueue.create(metalDevice)
    private val hostedViews = mutableSetOf<MetalViewContext>()

    fun createMetalViewContext(onDisplayLayer: () -> Unit = {}): MetalViewContext {
        val callbackReference = CallbackReference(onDisplayLayer)
        val view = MetalView.create(metalDevice, onDisplayLayer = { callbackReference.callback() })
        val directContext = DirectContext.makeMetal(metalDevice.pointerAddress, metalCommandQueue.pointerAddress)
        val viewContext = MetalViewContext(view, callbackReference, directContext, metalCommandQueue)
        check(hostedViews.add(viewContext)) { "View already exists" }
        return viewContext
    }

    fun destroyMetalViewContext(viewContext: MetalViewContext) {
        check(hostedViews.remove(viewContext)) { "No such view" }
        viewContext.stopRenderThread()
        viewContext.directContext.close()
        viewContext.view.close()
    }

    override fun close() {
        check(hostedViews.isEmpty()) { "Can't destroy GpuContext, ${hostedViews.count()} views are still alive" }
        metalCommandQueue.close()
        metalDevice.close()
    }
}
