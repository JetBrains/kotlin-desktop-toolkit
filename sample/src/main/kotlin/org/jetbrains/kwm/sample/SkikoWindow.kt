package org.jetbrains.kwm.sample

import org.jetbrains.kwm.LogicalPoint
import org.jetbrains.kwm.LogicalSize
import org.jetbrains.kwm.PhysicalSize
import org.jetbrains.kwm.macos.*
import org.jetbrains.skia.*
import kotlin.time.TimeSource

abstract class SkikoWindow(device: MetalDevice,
                           val queue: MetalCommandQueue,
                           title: String,
                           position: LogicalPoint): AutoCloseable {

    val window = Window.create(origin = position, title = title)
    var displayLink = DisplayLink.create(window.screenId(), onNextFrame = {
        performDrawing()
    })

    val directContext = DirectContext.makeMetal(device.pointer.address(), queue.pointer.address())
    var view: MetalView = MetalView.create(device)
    val creationTime = TimeSource.Monotonic.markNow()

    init {
        window.attachView(view)
        performDrawing()
    }

    private fun updateDisplayLink(screenId: ScreenId) {
        val isRunning = displayLink.isRunning()
        displayLink.close()
        displayLink = DisplayLink.create(screenId, onNextFrame = {
            performDrawing()
        })
        displayLink.setRunning(isRunning)
    }

    open fun handleEvent(event: Event): EventHandlerResult {
        return when (event) {
            is Event.WindowResize -> {
                performDrawing()
                EventHandlerResult.Continue
            }
            is Event.WindowScreenChange -> {
                updateDisplayLink(screenId = event.newScreenId)
                EventHandlerResult.Continue
            }
            else ->EventHandlerResult.Continue
        }
    }

    fun performDrawing() {
        val size = view.size()
        view.nextTexture().use { texture ->
//             sleep(100) // uncomment this to check window resize quality
            BackendRenderTarget.makeMetal(size.width.toInt(), size.height.toInt(), texture.pointer.address()).use { renderTarget ->
                Surface.makeFromBackendRenderTarget(
                    context = directContext,
                    origin = SurfaceOrigin.TOP_LEFT,
                    colorFormat = SurfaceColorFormat.BGRA_8888,
                    colorSpace = null,
                    surfaceProps = null,
                    rt = renderTarget
                )!!.use { surface ->
                    val time = creationTime.elapsedNow().inWholeMilliseconds
                    surface.canvas.draw(PhysicalSize(surface.width.toDouble(), surface.height.toDouble()), time)
                    surface.flushAndSubmit()
                }
            }
            queue.commit()
            view.present()
        }
    }

    fun scale(): Float {
        return window.scaleFactor().toFloat()
    }

    abstract fun Canvas.draw(size: PhysicalSize, time: Long)

    override fun close() {
        displayLink.close()
        directContext.close()
        view.close()
        window.close()
    }
}